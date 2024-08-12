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
use sub_client::{
    abi::ton_app::{TonAppStorage, TonAppTx},
    bridge_types::MainnetAssetId,
};
use toner::ton::MsgAddress;

use super::TonNetworkSelector;

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(subcommand)]
    apps: Apps,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Apps {
    /// Register JettonApp
    FungibleNew {
        /// TON Network
        #[clap(long)]
        network: TonNetworkSelector,
        /// JettonApp contract address
        #[clap(long)]
        contract: MsgAddress,
        /// Asset name
        #[clap(long)]
        name: String,
        /// Asset symbol
        #[clap(long)]
        symbol: String,
        /// Precision
        #[clap(long)]
        precision: u8,
    },
    /// Register JettonApp with existing asset
    FungibleExisting {
        /// TON Network
        #[clap(long)]
        network: TonNetworkSelector,
        /// JettonApp contract address
        #[clap(long)]
        contract: MsgAddress,
        /// AssetId
        #[clap(long)]
        asset_id: MainnetAssetId,
        /// Asset precision
        #[clap(long)]
        precision: u8,
    },
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let sub = self.sub.get_signed_substrate().await?;
        if self.check_if_registered(&sub.storage().await?).await? {
            return Ok(());
        }
        let tx = sub.tx().await?;
        match &self.apps {
            Apps::FungibleNew {
                contract,
                name,
                symbol,
                precision,
                network,
            } => {
                tx.register_network(
                    network.network(),
                    TonAddress::new(contract.workchain_id as i8, contract.address.into()),
                    symbol.clone().into_bytes(),
                    name.clone().into_bytes(),
                    *precision,
                )
                .await?
            }
            Apps::FungibleExisting {
                contract,
                asset_id,
                precision,
                network,
            } => {
                tx.register_network_with_existing_asset(
                    network.network(),
                    TonAddress::new(contract.workchain_id as i8, contract.address.into()),
                    *asset_id,
                    *precision,
                )
                .await?
            }
        }
        Ok(())
    }

    async fn check_if_registered(&self, sub: &SubStorage<SoraConfig>) -> AnyResult<bool> {
        let (contract, registered) = match self.apps {
            Apps::FungibleNew { contract, .. } | Apps::FungibleExisting { contract, .. } => {
                let registered = sub.app_info().await?;
                (contract, registered)
            }
        };
        let contract = TonAddress::new(contract.workchain_id as i8, contract.address.into());
        if let Some((registered_network_id, registered_contract)) = registered {
            if registered_contract == contract {
                info!("App already registered");
            } else {
                info!(
                    "App already registered with different contract address: ({:?}) {:?} != {:?}",
                    registered_network_id, contract, registered_contract
                );
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
