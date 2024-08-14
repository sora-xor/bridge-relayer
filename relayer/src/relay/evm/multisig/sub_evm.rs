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

use crate::prelude::*;
use bridge_types::evm::OutboundCommitment;
use bridge_types::GenericNetworkId;
use evm_client::alloy::sol_types::SolValue;
use sp_core::{ecdsa, H256};
use std::time::Duration;
use sub_client::{abi::channel::MaxU32, sp_runtime::traits::Hash};

pub struct RelayBuilder {
    sender: Option<SubUnsignedClient<SoraConfig>>,
    receiver: Option<EvmClient>,
    channel: Option<EvmAddress>,
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
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_sender_client(mut self, sender: SubUnsignedClient<SoraConfig>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_receiver_client(mut self, receiver: EvmClient) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn with_signer(mut self, signer: Option<ecdsa::Pair>) -> Self {
        self.signer = signer;
        self
    }

    pub fn with_channel_contract(mut self, address: EvmAddress) -> Self {
        self.channel = Some(address);
        self
    }

    pub async fn build(self) -> AnyResult<Relay> {
        let sender = self.sender.expect("sender client is needed");
        let receiver = self.receiver.expect("receiver client is needed");
        let channel = self.channel.expect("inbound channel address is needed");
        let sub_network_id = sender.constants().network_id().await?;
        Ok(Relay {
            evm_network_id: GenericNetworkId::EVM(receiver.chain_id().await?.0.into()),
            sub_network_id,
            sub: sender,
            evm: receiver,
            channel,
            signer: self.signer,
        })
    }
}

#[derive(Clone)]
pub struct Relay {
    sub: SubUnsignedClient<SoraConfig>,
    evm: EvmClient,
    evm_network_id: GenericNetworkId,
    sub_network_id: GenericNetworkId,
    channel: EvmAddress,
    signer: Option<ecdsa::Pair>,
}

// Relays batches of messages from Substrate to Ethereum.
impl Relay {
    fn submit_message_gas(&self, messages_total_gas: u128) -> u128 {
        messages_total_gas.saturating_add(260000)
    }

    async fn inbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self.evm.channel(self.channel).batchNonce().call().await?;
        Ok(nonce.batchNonce as u64)
    }

    async fn outbound_channel_nonce(&self) -> AnyResult<u64> {
        Ok(self
            .sub
            .storage()
            .await?
            .outbound_nonce(self.evm_network_id)
            .await?)
    }

    pub fn prepare_evm_signed_message(msg: H256) -> H256 {
        let mut prefix = b"\x19Ethereum Signed Message:\n32".to_vec();
        prefix.extend(msg.as_bytes());
        sp_core::keccak_256(&prefix).into()
    }

    async fn send_commitment(
        &self,
        commitment: OutboundCommitment<MaxU32, MaxU32>,
        signed_message: H256,
    ) -> AnyResult<()> {
        let Ok(channel) = self.evm.signed_channel(self.channel) else {
            debug!("Don't have a relayer account private key, skipping commitment send");
            return Ok(());
        };
        let batch = Self::prepare_batch(&commitment);
        let messages_total_gas = commitment.total_max_gas;
        let approvals = self
            .sub
            .storage()
            .await?
            .approvals(self.evm_network_id, signed_message)
            .await?;
        let (v, r, s) = approvals
            .into_iter()
            .map(|(_, approval)| {
                (
                    approval.0[64],
                    approval.0[..32].try_into().unwrap(),
                    approval.0[32..64].try_into().unwrap(),
                )
            })
            .fold((vec![], vec![], vec![]), |mut vrs, (v, r, s)| {
                vrs.0.push(v + 27);
                vrs.1.push(r);
                vrs.2.push(s);
                vrs
            });
        let call = channel
            .submit(batch, v, r, s)
            .gas(self.submit_message_gas(messages_total_gas.as_u128()))
            .with_cloned_provider();

        debug!("Check submit messages");
        call.call().await?;
        debug!("Send submit messages");
        let tx = call.send().await?;
        debug!("Wait for confirmations submit messages: {:?}", tx.tx_hash());
        let tx = tx.get_receipt().await?;
        debug!("Submit messages: {:?}", tx);
        for log in tx.inner.logs() {
            if let Ok(log) = log.log_decode::<evm_client::abi::Channel::BatchDispatched>() {
                info!("Batch dispatched: {:?}", log);
            }
        }
        Ok(())
    }

    fn prepare_batch(
        commitment: &OutboundCommitment<MaxU32, MaxU32>,
    ) -> evm_client::abi::Channel::Batch {
        evm_client::abi::Channel::Batch {
            nonce: alloy::primitives::U256::from(commitment.nonce),
            // u128 should be enough for gas
            total_max_gas: alloy::primitives::U256::from(commitment.total_max_gas.as_u128()),
            messages: commitment
                .messages
                .iter()
                .map(|message| evm_client::abi::Channel::Message {
                    // u128 should be enough for gas
                    max_gas: alloy::primitives::U256::from(message.max_gas.as_u128()),
                    target: message.target.0.into(),
                    payload: message.payload.to_vec().into(),
                })
                .collect(),
        }
    }

    fn prepare_message_to_sign(&self, commitment: &OutboundCommitment<MaxU32, MaxU32>) -> H256 {
        let batch = Self::prepare_batch(&commitment);
        let batch_hash = sp_runtime::traits::Keccak256::hash(&batch.abi_encode());
        let message = sp_runtime::traits::Keccak256::hash_of(&(
            self.sub_network_id,
            self.evm_network_id,
            batch_hash,
        ));
        let message = Self::prepare_evm_signed_message(message);
        message
    }

    async fn approve_and_send_commitment(&self, commitment: GenericCommitment) -> AnyResult<()> {
        let GenericCommitment::EVM(bridge_types::evm::Commitment::Outbound(commitment)) =
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
            .should_send_commitment(self.evm_network_id, message)
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
        let peers = self
            .sub
            .storage()
            .await?
            .peers(self.evm_network_id)
            .await?
            .ok_or(sub_client::error::Error::NetworkNotRegistered(
                self.evm_network_id,
            ))?;
        Ok(peers.iter().any(|public| signer_public == *public))
    }

    #[instrument(skip(self), name = "evm_multisig_sub_evm")]
    pub async fn run(self) -> AnyResult<()> {
        if self.signer.is_some() && !self.is_peer().await? {
            return Err(anyhow::anyhow!("Provided signer key is not a peer"));
        }
        let mut interval = tokio::time::interval(Duration::from_secs(6));
        loop {
            interval.tick().await;
            let inbound_nonce = self.inbound_channel_nonce().await?;
            let outbound_nonce = self.outbound_channel_nonce().await?;
            if inbound_nonce >= outbound_nonce {
                if inbound_nonce > outbound_nonce {
                    error!(
                        "Inbound channel nonce is higher than outbound channel nonce: {} > {}",
                        inbound_nonce, outbound_nonce
                    );
                }
                continue;
            }
            for nonce in (inbound_nonce + 1)..=outbound_nonce {
                info!("Submit commitment: {nonce}");
                let offchain_data = self
                    .sub
                    .commitment_with_nonce(self.evm_network_id.into(), nonce)
                    .await?;
                self.approve_and_send_commitment(offchain_data.commitment)
                    .await?;
            }
        }
    }
}
