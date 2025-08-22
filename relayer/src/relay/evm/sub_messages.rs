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

use crate::ethereum::EthLogDecode;
use crate::ethereum::SignedClientInner;
use crate::ethereum::UnsignedClientInner;
use crate::prelude::*;
use crate::substrate::MaxU32;
use crate::substrate::{BlockNumberOrHash, UnboundedGenericCommitment};
use bridge_types::evm::OutboundCommitment;
use bridge_types::GenericNetworkId;
use bridge_types::U256;
use ethers::types::Address;
use ethereum_gen::ChannelHandler;
use ethers::abi::RawLog;
use ethers::abi::Tokenize;
use ethers::providers::Middleware;
use sp_core::{ecdsa, H256};
use std::time::Duration;

/// Builder for Substrate → EVM batch relay.
pub struct RelayBuilder {
    sender: Option<SubUnsignedClient<MainnetConfig>>,
    receiver: Option<EthUnsignedOrSignedClient>,
    channel: Option<Address>,
    signer: Option<ecdsa::Pair>,
}

impl Default for RelayBuilder {
    fn default() -> Self {
        Self {
            sender: None,
            receiver: None,
            channel: None,
            signer: None,
        }
    }
}

impl RelayBuilder {
    /// Create a default builder.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_sender_client(mut self, sender: SubUnsignedClient<MainnetConfig>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_receiver_client(mut self, receiver: EthUnsignedOrSignedClient) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn with_signer(mut self, signer: Option<ecdsa::Pair>) -> Self {
        self.signer = signer;
        self
    }

    pub fn with_channel_contract(mut self, address: Address) -> Self {
        self.channel = Some(address);
        self
    }

    /// Finalize the builder and return a `Relay` instance.
    pub async fn build(self) -> AnyResult<Relay> {
        let sender = self.sender.expect("sender client is needed");
        let receiver = self.receiver.expect("receiver client is needed");
        let channel_address = self.channel.expect("inbound channel address is needed");
        let inbound_channel = receiver.as_ref().map_either(
            |l| ChannelHandler::new(channel_address, l.inner()),
            |r| ChannelHandler::new(channel_address, r.inner()),
        );
        let sub_network_id = sender.constant_fetch_or_default(
            &runtime::constants()
                .bridge_inbound_channel()
                .this_network_id(),
        )?;
        Ok(Relay {
            evm_network_id: either::for_both!(&receiver, r => r.chainid().await?.into()),
            sub_network_id,
            sub: sender,
            evm: receiver,
            inbound_channel,
            signer: self.signer,
        })
    }
}

/// Substrate → EVM batch relay.
#[derive(Clone)]
pub struct Relay {
    sub: SubUnsignedClient<MainnetConfig>,
    evm: EthUnsignedOrSignedClient,
    inbound_channel: Either<ChannelHandler<UnsignedClientInner>, ChannelHandler<SignedClientInner>>,
    evm_network_id: GenericNetworkId,
    sub_network_id: GenericNetworkId,
    signer: Option<ecdsa::Pair>,
}

// Relays batches of messages from Substrate to Ethereum.
impl Relay {
    /// Apply overhead to the sum of message gas costs for submission.
    fn submit_message_gas(&self, messages_total_gas: U256) -> U256 {
        messages_total_gas.saturating_add(260000.into())
    }

    /// Read current inbound channel nonce from EVM.
    async fn inbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = either::for_both!(&self.inbound_channel, c => c.batch_nonce().call().await?);
        Ok(nonce as u64)
    }

    /// Read current outbound channel nonce from Substrate.
    async fn outbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self
            .sub
            .storage_fetch_or_default(
                &mainnet_runtime::storage()
                    .bridge_outbound_channel()
                    .channel_nonces(&self.evm_network_id),
                BlockNumberOrHash::Finalized,
            )
            .await?;
        Ok(nonce)
    }

    /// Produce an Ethereum Signed Message pre-hash for ECDSA approvals.
    pub fn prepare_evm_signed_message(msg: H256) -> H256 {
        let mut prefix = b"\x19Ethereum Signed Message:\n32".to_vec();
        prefix.extend(msg.as_bytes());
        sp_core::keccak_256(&prefix).into()
    }

    /// Send a commitment to the inbound EVM channel with approvals.
    async fn send_commitment(
        &self,
        commitment: OutboundCommitment<MaxU32, MaxU32>,
        signed_message: H256,
    ) -> AnyResult<()> {
        let (Some(evm), Some(channel)) = (
            self.evm.as_ref().right(),
            self.inbound_channel.as_ref().right(),
        ) else {
            log::debug!("Don't have a relayer account private key, skipping commitment send");
            return Ok(());
        };
        let batch = Self::prepare_batch(&commitment);
        let messages_total_gas = commitment.total_max_gas;
        let approvals = self
            .sub
            .bridge_approvals(&self.evm_network_id, signed_message)
            .await?;
        let (v, r, s) = Self::approvals_to_vrs(approvals);
        let mut call: ethers::contract::ContractCall<_, ()> =
            channel.submit(batch, v, r, s).legacy();

        debug!("Fill submit messages");
        evm.fill_transaction(&mut call.tx, call.block).await?;
        debug!("Messages total gas: {}", messages_total_gas);
        call.tx.set_gas(self.submit_message_gas(messages_total_gas));
        debug!("Check submit messages");
        call.call().await?;
        evm.save_gas_price(&call, "submit-messages").await?;
        debug!("Send submit messages");
        let tx = call.send().await?;
        debug!("Wait for confirmations submit messages: {:?}", tx);
        let tx = tx.confirmations(1).await?;
        debug!("Submit messages: {:?}", tx);
        if let Some(tx) = tx {
            for log in tx.logs {
                let raw_log = RawLog {
                    topics: log.topics.clone(),
                    data: log.data.to_vec(),
                };
                if let Ok(log) =
                    <ethereum_gen::channel_handler::BatchDispatchedFilter as EthLogDecode>::decode_log(&raw_log)
                {
                    info!("Batch dispatched: {:?}", log);
                }
            }
        }
        Ok(())
    }

    fn approvals_to_vrs(
        approvals: Vec<sp_core::ecdsa::Signature>,
    ) -> (Vec<u8>, Vec<[u8; 32]>, Vec<[u8; 32]>) {
        approvals
            .into_iter()
            .map(|approval| {
                (
                    approval.0[64],
                    approval.0[..32].try_into().expect("slice to array"),
                    approval.0[32..64].try_into().expect("slice to array"),
                )
            })
            .fold((vec![], vec![], vec![]), |mut vrs, (v, r, s)| {
                vrs.0.push(v + 27);
                vrs.1.push(r);
                vrs.2.push(s);
                vrs
            })
    }

    #[inline]
    #[cfg(test)]
    fn pending_nonces(inbound_nonce: u64, outbound_nonce: u64) -> Vec<u64> {
        if inbound_nonce >= outbound_nonce {
            vec![]
        } else {
            ((inbound_nonce + 1)..=outbound_nonce).collect()
        }
    }

    fn prepare_batch(
        commitment: &OutboundCommitment<MaxU32, MaxU32>,
    ) -> ethereum_gen::channel_handler::Batch {
        ethereum_gen::channel_handler::Batch {
            nonce: commitment.nonce.into(),
            total_max_gas: commitment.total_max_gas.into(),
            messages: commitment
                .messages
                .iter()
                .map(|message| ethereum_gen::channel_handler::Message {
                    max_gas: message.max_gas.into(),
                    target: message.target.into(),
                    payload: message.payload.to_vec().into(),
                })
                .collect(),
        }
    }

    fn prepare_message_to_sign(&self, commitment: &OutboundCommitment<MaxU32, MaxU32>) -> H256 {
        let batch = Self::prepare_batch(&commitment);

        let tokens = batch.clone().into_tokens();
        let tokens = ethers::abi::Token::Tuple(tokens);
        let encoded_batch = ethers::abi::encode(&[tokens]);
        let batch_hash = sp_runtime::traits::Keccak256::hash(&encoded_batch);
        let message = sp_runtime::traits::Keccak256::hash_of(&(
            self.sub_network_id,
            self.evm_network_id,
            batch_hash,
        ));
        let message = Self::prepare_evm_signed_message(message);
        message
    }

    async fn approve_and_send_commitment(
        &self,
        commitment: UnboundedGenericCommitment,
    ) -> AnyResult<()> {
        let UnboundedGenericCommitment::EVM(bridge_types::evm::Commitment::Outbound(commitment)) =
            commitment
        else {
            return Err(anyhow::anyhow!(
                "Invalid commitment. EVM outbound commitment is expected"
            ));
        };
        let message = self.prepare_message_to_sign(&commitment);
        if let Some(signer) = &self.signer {
            self.sub
                .approve_message(signer.clone(), self.evm_network_id, message)
                .await?;
        }
        if self
            .sub
            .should_send_commitment(&self.evm_network_id, message)
            .await?
        {
            self.send_commitment(commitment, message).await?;
        }
        Ok(())
    }

    fn signer_public(&self) -> AnyResult<ecdsa::Public> {
        let signer_public = self
            .signer
            .as_ref()
            .map(|x| x.public())
            .ok_or(anyhow::anyhow!("No signer"))?;
        Ok(signer_public)
    }

    async fn is_peer(&self) -> AnyResult<bool> {
        let signer_public = self.signer_public()?;
        let peers = self.sub.bridge_peers(&self.evm_network_id).await?;
        Ok(peers.iter().any(|public| signer_public == *public))
    }

    pub async fn run(self) -> AnyResult<()> {
        if self.signer.is_some() && !self.is_peer().await? {
            return Err(anyhow::anyhow!("Provided signer key is not a peer"));
        }
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

            let res: AnyResult<()> = async {
                let inbound_nonce = self.inbound_channel_nonce().await?;
                let outbound_nonce = self.outbound_channel_nonce().await?;
                if inbound_nonce >= outbound_nonce {
                    if inbound_nonce > outbound_nonce {
                        error!(
                            "Inbound channel nonce is higher than outbound channel nonce: {} > {}",
                            inbound_nonce, outbound_nonce
                        );
                    }
                    return Ok(());
                }
                info!(
                    "Submit commitments from {} to {}",
                    inbound_nonce, outbound_nonce
                );
                for nonce in (inbound_nonce + 1)..=outbound_nonce {
                    let offchain_data = self
                        .sub
                        .commitment_with_nonce(
                            self.evm_network_id.into(),
                            nonce,
                            BlockNumberOrHash::Finalized,
                        )
                        .await?;
                    self.approve_and_send_commitment(offchain_data.commitment)
                        .await?;
                }
                Ok(())
            }
            .await;

            match res {
                Ok(()) => backoff_secs = 1,
                Err(e) => {
                    warn!("EVM relay iteration failed (will retry with backoff): {:?}", e);
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
    fn test_prepare_evm_signed_message() {
        let msg = H256::from_low_u64_be(0x1234_5678);
        let expected = {
            let mut p = b"\x19Ethereum Signed Message:\n32".to_vec();
            p.extend(msg.as_bytes());
            H256(sp_core::keccak_256(&p))
        };
        assert_eq!(Relay::prepare_evm_signed_message(msg), expected);
    }

    // (duplicate removed) test_prepare_batch_mapping

    #[test]
    fn test_prepare_batch_mapping() {
        use bridge_types::evm::{Message as EvmMsg, OutboundCommitment};
        use sp_runtime::BoundedVec;

        let commitment = OutboundCommitment::<MaxU32, MaxU32> {
            nonce: 7,
            total_max_gas: U256::from(12345u64),
            messages: sp_runtime::BoundedVec::truncate_from(vec![
                EvmMsg {
                    max_gas: U256::from(100u64),
                    target: Address::from_slice(&[0x11u8; 20]),
                    payload: BoundedVec::truncate_from(vec![1u8, 2, 3, 4]),
                },
                EvmMsg {
                    max_gas: U256::from(200u64),
                    target: Address::from_slice(&[0x22u8; 20]),
                    payload: BoundedVec::truncate_from(vec![5u8, 6, 7, 8]),
                },
            ]),
        };
        let batch = Relay::prepare_batch(&commitment);
        assert_eq!(batch.nonce, 7u64.into());
        assert_eq!(batch.total_max_gas, U256::from(12345u64).into());
        assert_eq!(batch.messages.len(), 2);
        assert_eq!(batch.messages[0].max_gas, U256::from(100u64).into());
        assert_eq!(batch.messages[0].target, Address::from_slice(&[0x11u8; 20]).into());
        assert_eq!(batch.messages[0].payload, ethers::types::Bytes::from(vec![1u8, 2, 3, 4]));
        assert_eq!(batch.messages[1].max_gas, U256::from(200u64).into());
        assert_eq!(batch.messages[1].target, Address::from_slice(&[0x22u8; 20]).into());
        assert_eq!(batch.messages[1].payload, ethers::types::Bytes::from(vec![5u8, 6, 7, 8]));
    }

    #[test]
    fn test_pending_nonces_edges() {
        assert_eq!(Relay::pending_nonces(5, 5), Vec::<u64>::new());
        assert_eq!(Relay::pending_nonces(6, 5), Vec::<u64>::new());
        assert_eq!(Relay::pending_nonces(4, 5), vec![5]);
        assert_eq!(Relay::pending_nonces(3, 5), vec![4, 5]);
    }

    #[test]
    fn test_approvals_to_vrs() {
        let mut sig1 = [0u8; 65];
        sig1[0..32].copy_from_slice(&[0xAA; 32]);
        sig1[32..64].copy_from_slice(&[0xBB; 32]);
        sig1[64] = 0;
        let mut sig2 = [0u8; 65];
        sig2[0..32].copy_from_slice(&[0x11; 32]);
        sig2[32..64].copy_from_slice(&[0x22; 32]);
        sig2[64] = 1;
        let approvals = vec![sp_core::ecdsa::Signature::from_raw(sig1), sp_core::ecdsa::Signature::from_raw(sig2)];
        let (v, r, s) = Relay::approvals_to_vrs(approvals);
        assert_eq!(v, vec![27u8, 28u8]);
        assert_eq!(r[0], [0xAA; 32]);
        assert_eq!(s[0], [0xBB; 32]);
        assert_eq!(r[1], [0x11; 32]);
        assert_eq!(s[1], [0x22; 32]);
    }

    // Omitted submit_message_gas test to avoid unsafe instance creation; the logic is a constant add.
}
