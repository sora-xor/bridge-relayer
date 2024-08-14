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

use crate::prelude::*;
use bridge_types::{GenericNetworkId, SubNetworkId, H256};
use sp_core::ecdsa;
use sp_runtime::traits::Keccak256;
use sub_client::abi::channel::ChannelStorage;
use sub_client::sp_runtime::traits::Hash;
use sub_client::types::BlockNumberOrHash;

pub struct RelayBuilder<S: subxt::Config, R: subxt::Config> {
    sender: Option<SubUnsignedClient<S>>,
    receiver: Option<SubUnsignedClient<R>>,
    signer: Option<ecdsa::Pair>,
}

impl<S: subxt::Config, R: subxt::Config> Default for RelayBuilder<S, R> {
    fn default() -> Self {
        Self {
            sender: None,
            receiver: None,
            signer: None,
        }
    }
}

impl<S, R> RelayBuilder<S, R>
where
    S: subxt::Config,
    R: subxt::Config,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_sender_client(mut self, sender: SubUnsignedClient<S>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_receiver_client(mut self, receiver: SubUnsignedClient<R>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn with_signer(mut self, signer: ecdsa::Pair) -> Self {
        self.signer = Some(signer);
        self
    }

    pub async fn build(self) -> AnyResult<Relay<S, R>> {
        let sender = self.sender.expect("sender client is needed");
        let receiver = self.receiver.expect("receiver client is needed");
        let signer = self.signer.expect("signer is needed");
        let sender_network_id = sender.constants().network_id().await?;

        let GenericNetworkId::Sub(sender_network_id) = sender_network_id else {
            return Err(anyhow::anyhow!("Error! Sender is NOT a Substrate Network!"));
        };

        let receiver_network_id = receiver.constants().network_id().await?;

        let GenericNetworkId::Sub(receiver_network_id) = receiver_network_id else {
            return Err(anyhow::anyhow!(
                "Error! Reciever is NOT a Substrate Network!"
            ));
        };

        Ok(Relay {
            sender,
            receiver,
            signer,
            receiver_network_id,
            sender_network_id,
        })
    }
}

#[derive(Clone)]
pub struct Relay<S: subxt::Config, R: subxt::Config> {
    sender: SubUnsignedClient<S>,
    receiver: SubUnsignedClient<R>,
    signer: ecdsa::Pair,
    receiver_network_id: SubNetworkId,
    sender_network_id: SubNetworkId,
}

impl<S, R> Relay<S, R>
where
    S: subxt::Config,
    R: subxt::Config,
    SubStorage<R>: ChannelStorage<R>,
    SubStorage<S>: ChannelStorage<S>,
    SubUnsignedTxs<R>: ChannelUnsignedTx<R> + MultisigUnsignedTx<R>,
    SubUnsignedTxs<S>: ChannelUnsignedTx<S> + MultisigUnsignedTx<S>,
    sub_client::BlockNumberOf<R>: Send + Sync + Decode,
    sub_client::BlockNumberOf<S>: Send + Sync + Decode,
{
    async fn inbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self
            .receiver
            .at(BlockNumberOrHash::Best)
            .await?
            .storage()
            .await?
            .inbound_nonce(self.sender_network_id.into())
            .await?;
        Ok(nonce)
    }

    async fn outbound_channel_nonce(&self) -> AnyResult<u64> {
        let nonce = self
            .sender
            .storage()
            .await?
            .outbound_nonce(self.receiver_network_id.into())
            .await?;
        Ok(nonce)
    }

    async fn approvals(&self, message: H256) -> AnyResult<Vec<ecdsa::Signature>> {
        let peers = self.receiver_peers().await?;
        let approvals = self
            .sender
            .storage()
            .await?
            .approvals(self.receiver_network_id.into(), message)
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

    async fn sender_peers(&self) -> AnyResult<BTreeSet<ecdsa::Public>> {
        let peers = self
            .sender
            .storage()
            .await?
            .peers(self.receiver_network_id.into())
            .await?
            .ok_or(SubError::NetworkNotRegistered(
                self.receiver_network_id.into(),
            ))?;
        Ok(peers)
    }

    async fn receiver_peers(&self) -> AnyResult<BTreeSet<ecdsa::Public>> {
        let peers = self
            .receiver
            .storage()
            .await?
            .peers(self.sender_network_id.into())
            .await?
            .ok_or(SubError::NetworkNotRegistered(
                self.receiver_network_id.into(),
            ))?;
        Ok(peers)
    }

    #[instrument(skip(self), name = "sub_multisig")]
    pub async fn run(self) -> AnyResult<()> {
        loop {
            let public = self.signer.public();
            let peers = self.sender_peers().await?;
            if !peers.contains(&public) {
                info!("Peer is not in trusted list, waiting...");
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            } else {
                break;
            }
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6));
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
                let offchain_data = self
                    .sender
                    .commitment_with_nonce(self.receiver_network_id.into(), nonce)
                    .await?;
                let commitment_hash = offchain_data.commitment.hash();
                let digest = self
                    .sender
                    .load_digest(
                        self.receiver_network_id.into(),
                        offchain_data.block_number.into(),
                        commitment_hash,
                    )
                    .await?;
                let digest_hash = Keccak256::hash_of(&digest);
                trace!("Digest hash: {}", digest_hash);
                if self
                    .sender
                    .should_send_approval(
                        self.receiver_network_id.into(),
                        self.signer.public(),
                        digest_hash,
                    )
                    .await?
                {
                    self.sender
                        .approve_message(
                            self.signer.clone(),
                            self.receiver_network_id.into(),
                            digest_hash,
                        )
                        .await?;
                }
                if self
                    .sender
                    .should_send_commitment(self.receiver_network_id.into(), digest_hash)
                    .await?
                {
                    let approvals = self.approvals(digest_hash).await?;
                    self.receiver
                        .unsigned_tx()
                        .await?
                        .submit(
                            self.sender_network_id.into(),
                            offchain_data.commitment,
                            sub_client::abi::channel::MultiProof::Sub(
                                sub_client::abi::channel::SubProof {
                                    digest,
                                    proof: approvals,
                                },
                            ),
                        )
                        .await?;
                } else {
                    info!(
                    "Still not enough signatures, probably another relayer will submit commitment"
                );
                    continue;
                }
            }
        }
    }
}
