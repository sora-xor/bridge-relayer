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
use bridge_types::{types::AssetKind, EVMChainId, H160};
use common::ETH;

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    eth: EthereumClient,
    #[clap(subcommand)]
    apps: Apps,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Apps {
    /// Register ERC20App
    FungibleApp {
        /// ERC20App contract address
        #[clap(long)]
        contract: H160,
    },
    /// Register EthApp with predefined ETH asset id
    EthAppPredefined {
        /// EthApp contract address
        #[clap(long)]
        contract: H160,
        /// ETH precision
        #[clap(long)]
        precision: u8,
    },
    // / Register EthApp with creating new ETH asset
    // EthAppNew {
    //     /// EthApp contract address
    //     #[clap(long)]
    //     contract: H160,
    //     /// ETH asset name
    //     #[clap(long)]
    //     name: String,
    //     /// ETH asset symbol
    //     #[clap(long)]
    //     symbol: String,
    //     /// ETH precision
    //     #[clap(long)]
    //     precision: u8,
    // },
    // /// Register EthApp with existing ETH asset
    // EthAppExisting {
    //     /// EthApp contract address
    //     #[clap(long)]
    //     contract: H160,
    //     /// ETH asset id
    //     #[clap(long)]
    //     asset_id: AssetId,
    //     /// ETH precision
    //     #[clap(long)]
    //     precision: u8,
    // },
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let eth = self.eth.get_unsigned_ethereum().await?;
        let sub = self.sub.get_signed_substrate().await?;
        let network_id = eth.chainid().await?;
        if self.check_if_registered(&sub, network_id).await? {
            return Ok(());
        }
        let call = match &self.apps {
            Apps::FungibleApp { contract } => {
                runtime::runtime_types::framenode_runtime::RuntimeCall::EVMFungibleApp(
                    runtime::runtime_types::evm_fungible_app::pallet::Call::register_fungible_app {
                        network_id,
                        contract: *contract,
                    },
                )
            }
            Apps::EthAppPredefined {
                contract,
                precision,
            } => runtime::runtime_types::framenode_runtime::RuntimeCall::EVMFungibleApp(
                runtime::runtime_types::evm_fungible_app::pallet::Call::register_native_app {
                    network_id,
                    contract: *contract,
                    asset_id: ETH.into(),
                    sidechain_precision: *precision,
                },
            ),
        };
        info!("Sudo call extrinsic: {:?}", call);
        sub.submit_extrinsic(&runtime::tx().sudo().sudo(call))
            .await?;
        Ok(())
    }

    async fn check_if_registered(
        &self,
        sub: &SubSignedClient<MainnetConfig>,
        network_id: EVMChainId,
    ) -> AnyResult<bool> {
        let (contract, registered) = match self.apps {
            Apps::FungibleApp { contract } => {
                let registered = sub
                    .storage_fetch(
                        &mainnet_runtime::storage()
                            .evm_fungible_app()
                            .app_addresses(&network_id, &AssetKind::Sidechain),
                        (),
                    )
                    .await?;
                (contract, registered)
            }
            Apps::EthAppPredefined { contract, .. } => {
                let registered = sub
                    .storage_fetch(
                        &mainnet_runtime::storage()
                            .evm_fungible_app()
                            .app_addresses(&network_id, &AssetKind::Native),
                        (),
                    )
                    .await?;
                (contract, registered)
            }
        };
        if let Some(registered) = registered {
            if registered == contract {
                info!("App already registered");
            } else {
                info!(
                    "App already registered with different contract address: {} != {}",
                    contract, registered
                );
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
