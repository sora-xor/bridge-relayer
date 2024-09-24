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

use sp_core::H160;
use sp_runtime::AccountId32;
use sub_client::{
    abi::evm_app::EvmAppStorage,
    bridge_types::{EVMChainId, MainnetAssetId},
};

use crate::cli::prelude::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    eth: EvmClientCli,
    /// Signer for bridge messages
    #[clap(long)]
    account_id: AccountId32,
    #[clap(long)]
    asset_id: MainnetAssetId,
    #[clap(long)]
    amount: u128,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let eth = self.eth.get_signed_evm().await?;
        let sub = self.sub.get_unsigned_substrate().await?;
        let chain_id: EVMChainId = eth.chain_id().await?.0.into();
        debug!("Eth chain id = {}", chain_id);
        let Some(_channel_address) = sub
            .storage()
            .await?
            .evm_channel_address(chain_id.into())
            .await?
        else {
            return Err(anyhow!("Bridge channel not registered"));
        };
        let Some(app_address) = sub.storage().await?.app_address(chain_id).await? else {
            return Err(anyhow!("Bridge app not registered"));
        };
        let Some(asset_address) = sub
            .storage()
            .await?
            .address_by_asset(chain_id, self.asset_id)
            .await?
        else {
            return Err(anyhow!("Asset not registered"));
        };
        let app = eth.signed_fungible_app(app_address.0.into()).await?;
        let amount = if asset_address == H160::zero() {
            0
        } else {
            self.amount
        };
        let call = app
            .lock(
                asset_address.0.into(),
                alloy::primitives::B256::new(*self.account_id.as_ref()),
                alloy::primitives::U256::from(amount),
            )
            .with_cloned_provider();
        info!("Static call");
        call.call().await?;
        info!("Submit transaction");
        let pending = call.send().await?;
        info!("Wait for confirmations: {:?}", pending);
        let receipt = pending.get_receipt().await?;
        info!("Result: {:?}", receipt);
        Ok(())
    }
}
