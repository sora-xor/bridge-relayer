// This file is part of the SORA network and Polkaswap app.

// Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
// SPDX-License-Identifier: BSD-4-Clause

// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:

// Redistributions of source code must retain the above copyright notice, this list
// of conditions and the following disclaimer.
// Redistributions in binary form must reproduce the above copyright notice, this
// list of conditions and the following disclaimer in the documentation and/or other
// materials provided with the distribution.
//
// All advertising materials mentioning features or use of this software must display
// the following acknowledgement: This product includes software developed by Polka Biome
// Ltd., SORA, and Polkaswap.
//
// Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used
// to endorse or promote products derived from this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Substrate → TON batch relay.
//!
//! Mirrors the Substrate → EVM relay but targets TON channel contract.
//! - Reads outbound commitments for `GenericNetworkId::TON(..)` from Substrate.
//! - Compares Substrate outbound nonce with TON inbound channel nonce.
//! - For each missing nonce, submits `SendInboundMessage` to the TON channel
//!   for every message in the commitment.

use crate::prelude::*;
use crate::substrate::{BlockNumberOrHash, UnboundedGenericCommitment};
use crate::ton::{SignedTonClient, TonClient};
use bridge_types::{ton::TonNetworkId, GenericNetworkId};
use num_bigint::BigUint;
use std::time::Duration;
use toner::ton::MsgAddress;
// no-op outbound path for now

/// Builder for Substrate → TON relay.
pub struct RelayBuilder {
    sub: Option<SubUnsignedClient<MainnetConfig>>,
    ton: Option<SignedTonClient>,
    ton_client_unsigned: Option<TonClient>,
    channel: Option<MsgAddress>,
    ton_network_id: Option<TonNetworkId>,
    /// Value in nanotons to attach to each message (gas/fees).
    value: BigUint,
    /// Whether to set bounce flag on messages.
    bounce: bool,
}

impl Default for RelayBuilder {
    fn default() -> Self {
        Self {
            sub: None,
            ton: None,
            ton_client_unsigned: None,
            channel: None,
            ton_network_id: None,
            // Safe default: 0.1 TON
            value: BigUint::from(100_000_000u64),
            bounce: true,
        }
    }
}

impl RelayBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_sub_client(mut self, sub: SubUnsignedClient<MainnetConfig>) -> Self {
        self.sub = Some(sub);
        self
    }

    pub fn with_signed_ton(mut self, ton: SignedTonClient) -> Self {
        self.ton = Some(ton);
        self
    }

    /// Optionally also provide an unsigned TON client to read inbound nonce.
    pub fn with_unsigned_ton(mut self, ton: TonClient) -> Self {
        self.ton_client_unsigned = Some(ton);
        self
    }

    pub fn with_channel(mut self, channel: MsgAddress) -> Self {
        self.channel = Some(channel);
        self
    }

    pub fn with_ton_network_id(mut self, ton_network_id: TonNetworkId) -> Self {
        self.ton_network_id = Some(ton_network_id);
        self
    }

    pub fn with_value(mut self, value: BigUint) -> Self {
        self.value = value;
        self
    }

    pub fn with_bounce(mut self, bounce: bool) -> Self {
        self.bounce = bounce;
        self
    }

    pub async fn build(self) -> AnyResult<Relay> {
        let sub = self
            .sub
            .ok_or_else(|| anyhow!("Substrate client is not provided"))?;
        let ton = self
            .ton
            .ok_or_else(|| anyhow!("Signed TON client is not provided"))?;
        let channel = self
            .channel
            .ok_or_else(|| anyhow!("TON channel address is not provided"))?;
        let ton_network_id = self
            .ton_network_id
            .ok_or_else(|| anyhow!("TON network id is not provided"))?;
        let sub_network_id = sub.constant_fetch_or_default(
            &runtime::constants()
                .bridge_outbound_channel()
                .this_network_id(),
        )?;
        Ok(Relay {
            sub,
            ton,
            ton_reader: self.ton_client_unsigned,
            channel,
            sub_network_id,
            ton_network_id: GenericNetworkId::TON(ton_network_id),
            value: self.value,
            bounce: self.bounce,
        })
    }
}

/// Substrate → TON relay.
pub struct Relay {
    sub: SubUnsignedClient<MainnetConfig>,
    ton: SignedTonClient,
    ton_reader: Option<TonClient>,
    channel: MsgAddress,
    // Substrate network id (used when building submissions; retained for future use)
    #[allow(dead_code)]
    sub_network_id: GenericNetworkId,
    ton_network_id: GenericNetworkId,
    value: BigUint,
    bounce: bool,
}

impl Relay {
    /// Build a TON Cell from payload bytes (bitstring), preserving bit order.
    fn payload_to_cell(payload: &[u8]) -> AnyResult<toner::tlb::Cell> {
        use toner::tlb::bits::ser::BitWriterExt;
        use subxt::ext::bitvec::view::AsBits;
        let mut builder = toner::tlb::Cell::builder();
        builder.pack(payload.as_bits())?;
        Ok(builder.into_cell())
    }

    #[inline]
    fn pending_nonces(inbound_nonce: u64, outbound_nonce: u64) -> Vec<u64> {
        if inbound_nonce >= outbound_nonce {
            vec![]
        } else {
            ((inbound_nonce + 1)..=outbound_nonce).collect()
        }
    }
    /// Read current inbound channel nonce from TON (if reader provided).
    async fn inbound_channel_nonce(&self) -> AnyResult<u64> {
        let Some(client) = &self.ton_reader else {
            // If no reader provided, assume zero to force sending from genesis.
            return Ok(0);
        };
        let res = client
            .run_get_method(self.channel, "inboundNonce", vec![], None)
            .await?;
        if res.exit_code == 0 {
            if let Some(crate::ton::types::StackEntry::Int(nonce)) = res.stack.get(0) {
                Ok(nonce.as_u64())
            } else {
                Err(anyhow!("Got wrong inbound nonce stack"))
            }
        } else {
            Err(anyhow!("Failed to fetch inbound nonce from channel"))
        }
    }

    /// Read current outbound channel nonce from Substrate.
    async fn outbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self
            .sub
            .storage_fetch_or_default(
                &runtime::storage()
                    .bridge_outbound_channel()
                    .channel_nonces(&self.ton_network_id),
                BlockNumberOrHash::Finalized,
            )
            .await?;
        Ok(nonce)
    }

    async fn approve_and_send_commitment(&self, commitment: UnboundedGenericCommitment) -> AnyResult<()> {
        use bridge_types::ton::Commitment as TonCommitment;
        use crate::ton::contracts::channel::SendInboundMessage;

        let UnboundedGenericCommitment::TON(commitment) = commitment else {
            return Err(anyhow::anyhow!("Invalid commitment. TON commitment is expected"));
        };

        let TonCommitment::Outbound(outbound) = commitment else {
            // Inbound commitment is not for Substrate→TON path
            info!("Skip non-outbound TON commitment");
            return Ok(());
        };

        // Iterate messages and submit each to the TON channel contract
        for msg in outbound.messages {
            // Map target address
            let target = MsgAddress {
                workchain_id: msg.target.workchain as i32,
                address: msg.target.address.0,
            };

            // Build payload cell from raw payload bits
            let payload_cell = Self::payload_to_cell(&msg.payload)?;

            // Respect per-message fee ceiling if our configured value exceeds it
            let max_fee: u128 = msg.max_fee.into();
            let configured: u128 = self.value.clone().try_into().unwrap_or(u128::MAX);
            let value = BigUint::from(configured.min(max_fee));

            // Prepare and send the message
            let body = SendInboundMessage {
                target,
                message: payload_cell,
            };
            let tx_hash = self
                .ton
                .submit(body, self.channel, value.clone(), self.bounce)
                .await?;
            info!("Submitted TON inbound message, tx={:?}", tx_hash);
        }

        Ok(())
    }

    pub async fn run(self) -> AnyResult<()> {
        // Basic retry/backoff to avoid tight error loops on transient failures.
        let mut backoff_secs: u64 = 1;
        let max_backoff: u64 = 30;
        let tick = Duration::from_secs(6);

        loop {
            if backoff_secs > 1 {
                debug!("retrying after {}s", backoff_secs);
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            } else {
                tokio::time::sleep(tick).await;
            }

            let run_res: AnyResult<()> = async {
                // Touch wallet to ensure clippy sees field usage; helpful for health-checks
                let _ = self.ton.seqno().await.ok();
                debug!("TON relay config: value={} bounce={}", self.value, self.bounce);
                let inbound_nonce = self.inbound_channel_nonce().await.unwrap_or(0);
                let outbound_nonce = self.outbound_channel_nonce().await?;
                let nonces = Self::pending_nonces(inbound_nonce, outbound_nonce);
                if nonces.is_empty() {
                    return Ok(());
                }
                info!("Submit TON commitments from {} to {}", inbound_nonce, outbound_nonce);
                for nonce in nonces {
                    let offchain = self
                        .sub
                        .commitment_with_nonce(self.ton_network_id, nonce, BlockNumberOrHash::Finalized)
                        .await?;
                    self.approve_and_send_commitment(offchain.commitment).await?;
                }
                Ok(())
            }
            .await;

            match run_res {
                Ok(()) => {
                    backoff_secs = 1;
                }
                Err(err) => {
                    warn!(
                        "TON relay iteration failed (will retry with backoff): {:?}",
                        err
                    );
                    backoff_secs = (backoff_secs.saturating_mul(2)).min(max_backoff);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_nonces_edges() {
        assert_eq!(Relay::pending_nonces(10, 10), Vec::<u64>::new());
        assert_eq!(Relay::pending_nonces(11, 10), Vec::<u64>::new());
        assert_eq!(Relay::pending_nonces(9, 10), vec![10]);
        assert_eq!(Relay::pending_nonces(8, 10), vec![9, 10]);
    }

    #[test]
    fn test_payload_to_cell_roundtrip() {
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let cell = Relay::payload_to_cell(&payload).expect("payload->cell");
        assert_eq!(cell.data.as_raw_slice(), payload.as_slice());
    }
}
