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

use bridge_types::evm::{BaseFeeUpdate, InboundCommitment, StatusReport};
use bridge_types::{EVMChainId, GenericNetworkId};
use sp_core::ecdsa;

use crate::prelude::*;
use crate::substrate::UnboundedGenericCommitment;
use ethers::prelude::*;

const BLOCKS_TO_INITIAL_SEARCH: u64 = 49000; // Ethereum light client keep 50000 blocks

pub struct SubstrateMessagesRelay {
    sub: SubUnsignedClient<MainnetConfig>,
    eth: EthUnsignedClient,
    evm_network_id: GenericNetworkId,
    sub_network_id: GenericNetworkId,
    channel: Address,
    latest_channel_block: u64,
    signer: ecdsa::Pair,
}

impl SubstrateMessagesRelay {
    pub async fn new(
        sub: SubUnsignedClient<MainnetConfig>,
        eth: EthUnsignedClient,
        signer: ecdsa::Pair,
    ) -> AnyResult<Self> {
        let chain_id = eth.chainid().await? as EVMChainId;
        let channel = sub
            .storage_fetch(
                &runtime::storage()
                    .bridge_inbound_channel()
                    .evm_channel_addresses(&chain_id),
                (),
            )
            .await?
            .ok_or(anyhow::anyhow!("Inbound channel is not registered"))?;
        let sub_network_id = sub.constant_fetch_or_default(
            &runtime::constants()
                .bridge_inbound_channel()
                .this_network_id(),
        )?;
        Ok(Self {
            latest_channel_block: 0,
            sub,
            eth,
            evm_network_id: chain_id.into(),
            sub_network_id,
            channel,
            signer,
        })
    }

    pub async fn handle_messages(&mut self) -> AnyResult<()> {
        let current_eth_block = self.eth.get_block_number().await?.as_u64();
        if current_eth_block < self.latest_channel_block {
            debug!("Skip handling channel messages, current block number is less than latest basic {} < {}", current_eth_block, self.latest_channel_block);
            return Ok(());
        }

        self.handle_message_events(current_eth_block).await?;
        self.handle_batch_dispatched(current_eth_block).await?;
        self.handle_base_fee_update(current_eth_block).await?;

        self.latest_channel_block = current_eth_block + 1;
        Ok(())
    }

    async fn handle_base_fee_update(&mut self, current_eth_block: u64) -> AnyResult<()> {
        let GenericNetworkId::EVM(chain_id) = self.evm_network_id else {
            unreachable!()
        };
        let old_base_fee = self
            .sub
            .storage_fetch(
                &runtime::storage().evm_fungible_app().base_fees(chain_id),
                (),
            )
            .await?
            .map(|x| x.base_fee)
            .unwrap_or(Default::default());
        let eth_block = current_eth_block - current_eth_block % 10;
        let block = self
            .eth
            .get_block(eth_block)
            .await?
            .ok_or(anyhow::anyhow!("Block {} not found", eth_block))?;
        let base_fee = block.base_fee_per_gas.unwrap_or_default();
        if base_fee != old_base_fee {
            let commitment = UnboundedGenericCommitment::EVM(
                bridge_types::evm::Commitment::BaseFeeUpdate(BaseFeeUpdate {
                    new_base_fee: base_fee,
                }),
            );
            info!("Submitting base fee update: {}", base_fee);
            self.submit_commitment(commitment).await?;
        }

        Ok(())
    }

    async fn handle_message_events(&mut self, current_eth_block: u64) -> AnyResult<()> {
        let eth = self.eth.inner();
        let channel = ethereum_gen::ChannelHandler::new(self.channel, eth.clone());
        let events: Vec<(
            ethereum_gen::channel_handler::MessageDispatchedFilter,
            LogMeta,
        )> = channel
            .message_dispatched_filter()
            .from_block(self.latest_channel_block)
            .to_block(current_eth_block)
            .query_with_meta()
            .await?;
        debug!(
            "Channel: Found {} Message events from {} to {}",
            events.len(),
            self.latest_channel_block,
            current_eth_block
        );
        let mut sub_nonce = self
            .sub
            .storage_fetch_or_default(
                &runtime::storage()
                    .bridge_inbound_channel()
                    .channel_nonces(&self.evm_network_id),
                (),
            )
            .await?;

        for (event, meta) in events {
            if event.nonce.as_u64() == sub_nonce + 1 && meta.address == self.channel {
                let commitment = UnboundedGenericCommitment::EVM(
                    bridge_types::evm::Commitment::Inbound(InboundCommitment {
                        source: meta.address,
                        block_number: meta.block_number.as_u64(),
                        nonce: event.nonce.as_u64(),
                        payload: event
                            .payload
                            .to_vec()
                            .try_into()
                            .map_err(|_| anyhow::anyhow!("Invalid payload"))?,
                    }),
                );
                info!("Submit commitment: {}", commitment.nonce());
                self.submit_commitment(commitment).await?;
                sub_nonce += 1;
            }
        }

        Ok(())
    }

    async fn submit_commitment(&self, commitment: UnboundedGenericCommitment) -> AnyResult<()> {
        let message = sp_runtime::traits::Keccak256::hash_of(&(
            self.evm_network_id,
            self.sub_network_id,
            commitment.hash(),
        ));
        if self.should_send_approval(message).await? {
            info!("Sending approval");
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
            info!("Sending commitment");
            let approvals = self.approvals(message).await?;
            let proof = VerifierMultiProof::EVMMultisig(
                runtime::runtime_types::multisig_verifier::MultiEVMProof {
                    proof: approvals.try_into().unwrap(),
                },
            );
            let success = self
                .sub
                .submit_concurrent_unsigned_extrinsic(
                    &runtime::tx().bridge_inbound_channel().submit(
                        self.evm_network_id,
                        commitment,
                        proof,
                    ),
                )
                .await?;
            if success {
                info!("Commitment submitted by this relayer");
            } else {
                info!("Commitment will be submitted by another relayer");
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
                    .multisig_verifier()
                    .peer_keys(&self.evm_network_id),
                (),
            )
            .await?
            .unwrap_or_default()
            .into_iter()
            .collect();
        Ok(peers)
    }

    async fn handle_batch_dispatched(&mut self, current_eth_block: u64) -> AnyResult<()> {
        let eth = self.eth.inner();
        let inbound_channel = ethereum_gen::ChannelHandler::new(self.channel, eth.clone());
        let events: Vec<(
            ethereum_gen::channel_handler::BatchDispatchedFilter,
            LogMeta,
        )> = inbound_channel
            .batch_dispatched_filter()
            .from_block(self.latest_channel_block)
            .to_block(current_eth_block)
            .query_with_meta()
            .await?;
        debug!(
            "Channel: Found {} BatchDispatched events from {} to {}",
            events.len(),
            self.latest_channel_block,
            current_eth_block
        );

        let mut sub_reported_nonce = self
            .sub
            .storage_fetch_or_default(
                &runtime::storage()
                    .bridge_inbound_channel()
                    .reported_channel_nonces(&self.evm_network_id),
                (),
            )
            .await?;

        for (event, meta) in events {
            if event.batch_nonce.as_u64() == sub_reported_nonce + 1 && meta.address == self.channel
            {
                let mut results = vec![];
                for i in 0..event.results_length.as_usize() {
                    if event.results.bit(i) {
                        results.push(true);
                    } else {
                        results.push(false);
                    }
                }
                let commitment = UnboundedGenericCommitment::EVM(
                    bridge_types::evm::Commitment::StatusReport(StatusReport {
                        nonce: event.batch_nonce.as_u64(),
                        base_fee: event.base_fee,
                        gas_spent: event.gas_spent,
                        relayer: event.relayer,
                        results: results.try_into().unwrap(),
                        source: meta.address,
                        block_number: meta.block_number.as_u64(),
                    }),
                );
                info!("Submitting status report: {:?}", commitment.nonce());
                self.submit_commitment(commitment).await?;
                sub_reported_nonce += 1;
            }
        }

        Ok(())
    }

    pub async fn run(mut self) -> AnyResult<()> {
        let current_eth_block = self.eth.get_block_number().await?.as_u64();
        self.latest_channel_block = current_eth_block.saturating_sub(BLOCKS_TO_INITIAL_SEARCH);
        loop {
            debug!("Handle channel messages");
            if let Err(err) = self.handle_messages().await {
                warn!("Failed to handle channel messages: {}", err);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
