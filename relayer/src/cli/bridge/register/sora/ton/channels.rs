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

use crate::cli::prelude::*;
use bridge_types::ton::TonAddress;
use sp_core::crypto::Ss58Codec;
use toner::ton::MsgAddress;

#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    /// Channel contract address
    #[clap(long)]
    channel: MsgAddress,
    /// Bridge peers
    #[clap(long)]
    peers: Vec<String>,
    /// TON network
    #[clap(long)]
    network: TonNetworkSelector,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let sub = self.sub.get_signed_substrate().await?;

        let network_id = self.network.network();

        let peers = self
            .peers
            .iter()
            .map(|peer| sp_core::ecdsa::Public::from_string(peer.as_str()))
            .try_fold(
                vec![],
                |mut acc, peer| -> AnyResult<Vec<sp_core::ecdsa::Public>> {
                    acc.push(peer?);
                    Ok(acc)
                },
            )?;

        let is_channel_registered = sub
            .storage_fetch(
                &mainnet_runtime::storage()
                    .bridge_inbound_channel()
                    .ton_channel_addresses(&network_id),
                (),
            )
            .await?
            .is_some();
        if !is_channel_registered {
            let call = runtime::runtime_types::framenode_runtime::RuntimeCall::BridgeInboundChannel(
                runtime::runtime_types::bridge_channel::inbound::pallet::Call::register_ton_channel {
                    network_id,
                    channel_address: TonAddress::new(self.channel.workchain_id as i8, self.channel.address.into()),
                },
            );
            info!("Sudo call extrinsic: {:?}", call);
            sub.submit_extrinsic(&runtime::tx().sudo().sudo(call))
                .await?;

            let network_id = bridge_types::GenericNetworkId::TON(network_id);
            let call = mainnet_runtime::runtime_types::framenode_runtime::RuntimeCall::BridgeDataSigner(
            mainnet_runtime::runtime_types::bridge_data_signer::pallet::Call::register_network {
                network_id,
                peers: peers.clone(),
            },
        );
            info!("Submit sudo call: {call:?}");
            let call = mainnet_runtime::tx().sudo().sudo(call);
            sub.submit_extrinsic(&call).await?;

            let call =
                mainnet_runtime::runtime_types::framenode_runtime::RuntimeCall::MultisigVerifier(
                    mainnet_runtime::runtime_types::multisig_verifier::pallet::Call::initialize {
                        network_id,
                        peers,
                    },
                );
            info!("Submit sudo call: {call:?}");
            let call = mainnet_runtime::tx().sudo().sudo(call);
            sub.submit_extrinsic(&call).await?;
        } else {
            info!("Channel already registered");
        }
        Ok(())
    }
}
