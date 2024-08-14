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

use std::time::Duration;

use bridge_types::evm::{InboundCommitment, StatusReport};
use bridge_types::GenericNetworkId;
use sp_core::ecdsa;
use sub_client::abi::channel::ChannelStorage;
use sub_client::types::BlockNumberOrHash;

use crate::prelude::*;

const BLOCKS_TO_INITIAL_SEARCH: u64 = 49000; // Ethereum light client keep 50000 blocks

pub struct SubstrateMessagesRelay {
    sub: SubUnsignedClient<SoraConfig>,
    eth: EvmClient,
    evm_network_id: GenericNetworkId,
    sub_network_id: GenericNetworkId,
    channel: EvmAddress,
    latest_channel_block: u64,
    signer: ecdsa::Pair,
}

impl SubstrateMessagesRelay {
    pub async fn new(
        sub: SubUnsignedClient<SoraConfig>,
        eth: EvmClient,
        signer: ecdsa::Pair,
    ) -> AnyResult<Self> {
        let chain_id = eth.chain_id().await?.0.into();
        let evm_network_id = GenericNetworkId::EVM(chain_id);
        let channel = sub
            .storage()
            .await?
            .evm_channel_address(evm_network_id)
            .await?
            .ok_or(anyhow::anyhow!("Inbound channel is not registered"))?;
        let sub_network_id = sub.constants().network_id().await?;
        Ok(Self {
            latest_channel_block: 0,
            sub,
            eth,
            evm_network_id: chain_id.into(),
            sub_network_id,
            channel: EvmAddress::new(channel.0),
            signer,
        })
    }

    #[instrument(err(level = "warn"), skip(self), fields(from, to))]
    pub async fn handle_messages(&mut self) -> AnyResult<()> {
        let current_eth_block = self.eth.get_finalized_block_number().await?;
        if current_eth_block < self.latest_channel_block {
            debug!(
                "Skip handling channel messages, current block number is less than latest handled"
            );
            return Ok(());
        }
        tracing::Span::current().record("to", current_eth_block);
        tracing::Span::current().record("from", self.latest_channel_block);
        debug!("Handle events",);

        self.handle_message_events(current_eth_block).await?;
        self.handle_batch_dispatched(current_eth_block).await?;
        // self.handle_base_fee_update(current_eth_block).await?;

        self.latest_channel_block = current_eth_block + 1;
        Ok(())
    }

    // async fn handle_base_fee_update(&mut self, current_eth_block: u64) -> AnyResult<()> {
    //     let GenericNetworkId::EVM(chain_id) = self.evm_network_id else {
    //         unreachable!()
    //     };
    //     let eth_block = current_eth_block - current_eth_block % 10;

    //     let last_update = self
    //         .sub
    //         .storage_fetch(
    //             &runtime::storage().evm_fungible_app().base_fees(chain_id),
    //             (),
    //         )
    //         .await?
    //         .map(|x| x.evm_block_number)
    //         .unwrap_or(Default::default());
    //     if eth_block <= last_update {
    //         info!("Skip base fee update, too early");
    //         return Ok(());
    //     }

    //     let block = self
    //         .eth
    //         .get_block(eth_block)
    //         .await?
    //         .ok_or(anyhow::anyhow!("Block {} not found", eth_block))?;
    //     let base_fee = block.base_fee_per_gas.unwrap_or_default();
    //     let commitment = UnboundedGenericCommitment::EVM(
    //         bridge_types::evm::Commitment::BaseFeeUpdate(BaseFeeUpdate {
    //             new_base_fee: base_fee,
    //             evm_block_number: eth_block,
    //         }),
    //     );
    //     info!("Submitting base fee update: {}", base_fee);
    //     self.sub
    //         .submit_inbound_commitment(
    //             self.signer.clone(),
    //             self.evm_network_id,
    //             self.sub_network_id,
    //             commitment,
    //         )
    //         .await?;
    //     Ok(())
    // }

    async fn handle_message_events(&mut self, current_eth_block: u64) -> AnyResult<()> {
        let channel = self.eth.channel(self.channel);
        let events = channel
            .MessageDispatched_filter()
            .from_block(self.latest_channel_block)
            .to_block(current_eth_block)
            .query()
            .await?;
        trace!("Channel: Found {} Message events", events.len(),);
        let mut sub_nonce = self
            .sub
            .at(BlockNumberOrHash::Best)
            .await?
            .storage()
            .await?
            .inbound_nonce(self.evm_network_id)
            .await?;

        for (event, meta) in events {
            if event.nonce.to::<u64>() == sub_nonce + 1 && meta.address() == self.channel {
                let commitment = GenericCommitment::EVM(bridge_types::evm::Commitment::Inbound(
                    InboundCommitment {
                        channel: meta.address().0 .0.into(),
                        source: event.source.0 .0.into(),
                        block_number: meta
                            .block_number
                            .ok_or(evm_client::error::Error::BlockNotFound)?,
                        nonce: event.nonce.to::<u64>(),
                        payload: event
                            .payload
                            .to_vec()
                            .try_into()
                            .map_err(|_| anyhow::anyhow!("Invalid payload"))?,
                    },
                ));
                info!("Submit commitment: {}", commitment.nonce());
                self.sub
                    .submit_inbound_commitment(
                        self.signer.clone(),
                        self.evm_network_id,
                        self.sub_network_id,
                        commitment,
                    )
                    .await?;
                sub_nonce += 1;
            }
        }

        Ok(())
    }

    async fn handle_batch_dispatched(&mut self, current_eth_block: u64) -> AnyResult<()> {
        let events = self
            .eth
            .channel(self.channel)
            .BatchDispatched_filter()
            .from_block(self.latest_channel_block)
            .to_block(current_eth_block)
            .query()
            .await?;
        trace!("Channel: Found {} BatchDispatched events", events.len(),);

        let mut sub_reported_nonce = self
            .sub
            .at(BlockNumberOrHash::Best)
            .await?
            .storage()
            .await?
            .reported_nonce(self.evm_network_id)
            .await?;

        for (event, meta) in events {
            if event.batch_nonce.to::<u64>() == sub_reported_nonce + 1
                && meta.address() == self.channel
            {
                let mut results = vec![];
                for i in 0..event.results_length.to::<usize>() {
                    if event.results.bit(i) {
                        results.push(true);
                    } else {
                        results.push(false);
                    }
                }
                let commitment = GenericCommitment::EVM(
                    bridge_types::evm::Commitment::StatusReport(StatusReport {
                        nonce: event.batch_nonce.to::<u64>(),
                        base_fee: sp_core::U256::from_little_endian(event.base_fee.as_le_slice()),
                        gas_spent: sp_core::U256::from_little_endian(event.gas_spent.as_le_slice()),
                        relayer: event.relayer.0 .0.into(),
                        results: results.try_into().unwrap(),
                        channel: meta.address().0 .0.into(),
                        block_number: meta
                            .block_number
                            .ok_or(evm_client::error::Error::BlockNotFound)?,
                    }),
                );
                info!("Submitting status report: {:?}", commitment.nonce());
                self.sub
                    .submit_inbound_commitment(
                        self.signer.clone(),
                        self.evm_network_id,
                        self.sub_network_id,
                        commitment,
                    )
                    .await?;
                sub_reported_nonce += 1;
            }
        }

        Ok(())
    }

    #[instrument(skip(self), name = "evm_multisig_evm_sub")]
    pub async fn run(mut self) -> AnyResult<()> {
        let current_eth_block = self.eth.get_finalized_block_number().await?;

        self.latest_channel_block = current_eth_block.saturating_sub(BLOCKS_TO_INITIAL_SEARCH);
        let events = self
            .eth
            .channel(self.channel)
            .Reseted_filter()
            .from_block(self.latest_channel_block)
            .to_block(current_eth_block)
            .query()
            .await?;
        self.latest_channel_block = events
            .into_iter()
            .filter_map(|(_log, meta)| meta.block_number)
            .max()
            .unwrap_or(self.latest_channel_block);
        let mut attempts = 0;
        loop {
            if let Err(err) = self.handle_messages().await {
                if attempts > 3 {
                    return Err(err);
                }
                attempts += 1;
                warn!("Failed to handle channel messages: {}", err);
            } else {
                attempts = 0;
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
