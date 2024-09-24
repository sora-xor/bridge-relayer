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
use bridge_types::{EVMChainId, H160};
use sub_client::{
    abi::evm_app::{EvmAppStorage, EvmAppTx},
    bridge_types::MainnetAssetId,
};

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    eth: EvmClientCli,
    #[clap(subcommand)]
    asset_kind: AssetKind,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AssetKind {
    /// Register ERC20 asset with given asset id
    ExistingERC20 {
        /// ERC20 asset id
        #[clap(long)]
        asset_id: MainnetAssetId,
        /// ERC20 token address
        #[clap(long)]
        address: H160,
        /// ERC20 token decimals
        #[clap(long)]
        decimals: u8,
    },
    /// Register ERC20 asset with creating new asset
    ERC20 {
        /// ERC20 token address
        #[clap(long)]
        address: H160,
        /// ERC20 asset name
        #[clap(long)]
        name: String,
        /// ERC20 asset symbol
        #[clap(long)]
        symbol: String,
        /// ERC20 asset decimals
        #[clap(long)]
        decimals: u8,
    },
    /// Register native asset with given asset id
    Native {
        /// Native asset id
        #[clap(long)]
        asset_id: MainnetAssetId,
    },
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let eth = self.eth.get_unsigned_evm().await?;
        let sub = self.sub.get_signed_substrate().await?;
        let network_id: EVMChainId = eth.chain_id().await?.0.into();
        if self
            .check_if_registered(&sub.storage().await?, network_id)
            .await?
        {
            return Ok(());
        }
        let tx = sub.tx().await?;
        match &self.asset_kind {
            AssetKind::ExistingERC20 {
                asset_id,
                address,
                decimals,
            } => {
                tx.register_existing_asset(network_id, *address, *asset_id, *decimals)
                    .await?
            }
            AssetKind::ERC20 {
                address,
                name,
                symbol,
                decimals,
            } => {
                tx.register_asset(
                    network_id,
                    *address,
                    symbol.clone().into_bytes(),
                    name.clone().into_bytes(),
                    *decimals,
                )
                .await?
            }
            AssetKind::Native { asset_id } => tx.register_sora_asset(network_id, *asset_id).await?,
        };
        Ok(())
    }

    pub async fn check_if_registered(
        &self,
        sub: &SubStorage<SoraConfig>,
        network_id: EVMChainId,
    ) -> AnyResult<bool> {
        let is_registered = match &self.asset_kind {
            AssetKind::ExistingERC20 { asset_id, .. } | AssetKind::Native { asset_id } => {
                sub.asset_kind(network_id, *asset_id).await?.is_some()
            }
            AssetKind::ERC20 { address, .. } => {
                sub.asset_by_address(network_id, *address).await?.is_some()
            }
        };
        if is_registered {
            info!("Asset is already registered");
        }
        Ok(is_registered)
    }
}
