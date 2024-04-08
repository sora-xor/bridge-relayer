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

// TODO #167: fix clippy warnings
#![allow(clippy::all)]

use std::collections::BTreeSet;
use std::time::Duration;

use crate::ethereum::EthLogDecode;
use crate::ethereum::SignedClientInner;
use crate::prelude::*;
use crate::substrate::{BlockNumberOrHash, UnboundedGenericCommitment};
use bridge_types::GenericNetworkId;
use bridge_types::{Address, U256};
use ethereum_gen::ChannelHandler;
use ethers::abi::RawLog;
use ethers::abi::Tokenize;
use ethers::providers::Middleware;
use sp_core::{ecdsa, H256};

pub struct RelayBuilder {
    sender: Option<SubUnsignedClient<MainnetConfig>>,
    receiver: Option<EthSignedClient>,
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
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_sender_client(mut self, sender: SubUnsignedClient<MainnetConfig>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_receiver_client(mut self, receiver: EthSignedClient) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn with_signer(mut self, signer: ecdsa::Pair) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_channel_contract(mut self, address: Address) -> Self {
        self.channel = Some(address);
        self
    }

    pub async fn build(self) -> AnyResult<Relay> {
        let sender = self.sender.expect("sender client is needed");
        let signer = self.signer.expect("signer is needed");
        let receiver = self.receiver.expect("receiver client is needed");
        let inbound_channel = ChannelHandler::new(
            self.channel.expect("inbound channel address is needed"),
            receiver.inner(),
        );
        let sub_network_id = sender.constant_fetch_or_default(
            &runtime::constants()
                .bridge_inbound_channel()
                .this_network_id(),
        )?;
        Ok(Relay {
            evm_network_id: receiver.chainid().await?.into(),
            sub_network_id,
            sub: sender,
            evm: receiver,
            inbound_channel,
            signer,
        })
    }
}

#[derive(Clone)]
pub struct Relay {
    sub: SubUnsignedClient<MainnetConfig>,
    evm: EthSignedClient,
    inbound_channel: ChannelHandler<SignedClientInner>,
    evm_network_id: GenericNetworkId,
    sub_network_id: GenericNetworkId,
    signer: ecdsa::Pair,
}

// Relays batches of messages from Substrate to Ethereum.
impl Relay {
    fn submit_message_gas(&self, messages_total_gas: U256) -> U256 {
        messages_total_gas.saturating_add(260000.into())
    }

    async fn inbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self.inbound_channel.batch_nonce().call().await?;
        Ok(nonce as u64)
    }

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

    pub fn prepare_message(msg: H256) -> H256 {
        let mut prefix = b"\x19Ethereum Signed Message:\n32".to_vec();
        prefix.extend(msg.as_bytes());
        sp_core::keccak_256(&prefix).into()
    }

    async fn submit_commitment(&self, commitment: UnboundedGenericCommitment) -> AnyResult<()> {
        let UnboundedGenericCommitment::EVM(bridge_types::evm::Commitment::Outbound(commitment)) =
            commitment
        else {
            return Err(anyhow::anyhow!(
                "Invalid commitment. EVM outbound commitment is expected"
            ));
        };
        let messages_total_gas = commitment.total_max_gas;
        let batch = ethereum_gen::channel_handler::Batch {
            nonce: commitment.nonce.into(),
            total_max_gas: commitment.total_max_gas.into(),
            messages: commitment
                .messages
                .into_iter()
                .map(|message| ethereum_gen::channel_handler::Message {
                    max_gas: message.max_gas.into(),
                    target: message.target.into(),
                    payload: message.payload.to_vec().into(),
                })
                .collect(),
        };

        let tokens = batch.clone().into_tokens();
        let tokens = ethers::abi::Token::Tuple(tokens);
        let encoded_batch = ethers::abi::encode(&[tokens]);
        let batch_hash = sp_runtime::traits::Keccak256::hash(&encoded_batch);
        let message = sp_runtime::traits::Keccak256::hash_of(&(
            self.sub_network_id,
            self.evm_network_id,
            batch_hash,
        ));
        let message = Self::prepare_message(message);
        if self.should_send_approval(message).await? {
            let signature = self.signer.sign_prehashed(&message.0);
            self.sub
                .submit_unsigned_extrinsic(&runtime::tx().bridge_data_signer().approve(
                    self.evm_network_id,
                    message,
                    signature,
                ))
                .await?;
        }
        if self.should_send_commitment(message).await? {
            let approvals = self.approvals(message).await?;
            let (v, r, s) = approvals
                .into_iter()
                .map(|approval| {
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
            let mut call: ethers::contract::ContractCall<_, ()> =
                self.inbound_channel.submit(batch, v, r, s).legacy();

            debug!("Fill submit messages");
            self.evm.fill_transaction(&mut call.tx, call.block).await?;
            debug!("Messages total gas: {}", messages_total_gas);
            call.tx.set_gas(self.submit_message_gas(messages_total_gas));
            debug!("Check submit messages");
            call.call().await?;
            self.evm.save_gas_price(&call, "submit-messages").await?;
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
        }
        Ok(())
    }

    async fn should_send_approval(&self, message: H256) -> AnyResult<bool> {
        let peers = self.receiver_peers().await?;
        let approvals = self.approvals(message).await?;
        let is_already_approved = approvals
            .iter()
            .filter_map(|approval| approval.recover_prehashed(&message.0))
            .any(|public| self.signer.public() == public);
        Ok(
            (approvals.len() as u32) < bridge_types::utils::threshold(peers.len() as u32)
                && !is_already_approved,
        )
    }

    async fn should_send_commitment(&self, message: H256) -> AnyResult<bool> {
        let peers = self.receiver_peers().await?;
        let approvals = self.approvals(message).await?;
        Ok((approvals.len() as u32) >= bridge_types::utils::threshold(peers.len() as u32))
    }

    async fn approvals(&self, message: H256) -> AnyResult<Vec<ecdsa::Signature>> {
        let peers = self.receiver_peers().await?;
        let approvals = self
            .sub
            .storage_fetch_or_default(
                &runtime::storage()
                    .bridge_data_signer()
                    .approvals(self.evm_network_id, message),
                (),
            )
            .await?;
        let mut acceptable_approvals = vec![];
        for approval in approvals {
            let public = approval
                .1
                .recover_prehashed(&message.0)
                .ok_or(anyhow!("Wrong signature in data signer pallet"))?;
            if peers.contains(&public) {
                acceptable_approvals.push(approval.1);
            }
        }
        Ok(acceptable_approvals)
    }

    async fn receiver_peers(&self) -> AnyResult<BTreeSet<ecdsa::Public>> {
        let peers = self
            .sub
            .storage_fetch(
                &runtime::storage()
                    .bridge_data_signer()
                    .peers(&self.evm_network_id),
                (),
            )
            .await?
            .unwrap_or_default()
            .into_iter()
            .collect();
        Ok(peers)
    }

    pub async fn run(self) -> AnyResult<()> {
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
                self.submit_commitment(offchain_data.commitment).await?;
            }
        }
    }
}
