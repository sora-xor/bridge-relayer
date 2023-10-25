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

use super::*;
use crate::prelude::*;
use clap::*;
use common::{DAI, PSWAP, VAL, XOR, XST, XSTUSD};

#[derive(Args, Clone, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(long)]
    mainnet_url: String,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let sub = self.sub.get_signed_substrate().await?;
        let mainnet = SubUnsignedClient::<MainnetConfig>::new(self.mainnet_url.clone()).await?;
        for (dex_id, base) in [(0, XOR), (1, XSTUSD)] {
            for asset_id in [XOR, PSWAP, DAI, XSTUSD, VAL, XST] {
                let reserves = mainnet
                    .api()
                    .storage()
                    .fetch(
                        &runtime::storage().pool_xyk().reserves(&base, &asset_id),
                        None,
                    )
                    .await?
                    .unwrap();
                let current_reserves = sub
                    .api()
                    .storage()
                    .fetch(
                        &runtime::storage().pool_xyk().reserves(&base, &asset_id),
                        None,
                    )
                    .await?
                    .unwrap();
                if reserves.0 <= 1
                    || reserves.1 <= 1
                    || current_reserves.0 > 1
                    || current_reserves.1 > 1
                {
                    continue;
                }
                info!("Add liquidity {}-{}: {:?}", base, asset_id, reserves);
                info!("Mint {}: {}", base, reserves.0 as i128 * 2);
                sub.submit_extrinsic(&runtime::tx().sudo().sudo(
                    runtime::runtime_types::framenode_runtime::RuntimeCall::Assets(
                        runtime::runtime_types::assets::pallet::Call::force_mint {
                            asset_id: base,
                            to: sub.account_id(),
                            amount: reserves.0 * 2,
                        },
                    ),
                ))
                .await?;
                info!("Mint {}: {}", asset_id, reserves.1 as i128 * 2);
                sub.submit_extrinsic(&runtime::tx().sudo().sudo(
                    runtime::runtime_types::framenode_runtime::RuntimeCall::Assets(
                        runtime::runtime_types::assets::pallet::Call::force_mint {
                            asset_id: asset_id,
                            to: sub.account_id(),
                            amount: reserves.1 * 2,
                        },
                    ),
                ))
                .await?;
                let tp = sub
                    .api()
                    .storage()
                    .fetch(
                        &runtime::storage().trading_pair().enabled_sources(
                            &dex_id,
                            &runtime::runtime_types::common::primitives::TradingPair {
                                base_asset_id: base,
                                target_asset_id: asset_id,
                            },
                        ),
                        None,
                    )
                    .await?;
                if tp.is_none() {
                    info!("Registering trading pair");
                    sub.submit_extrinsic(
                        &runtime::tx()
                            .trading_pair()
                            .register(dex_id, base, asset_id),
                    )
                    .await?;
                }
                info!("Initializing pool");
                sub.submit_extrinsic(
                    &runtime::tx()
                        .pool_xyk()
                        .initialize_pool(dex_id, base, asset_id),
                )
                .await?;
                info!("Deposit liquidity");
                sub.submit_extrinsic(
                    &runtime::tx()
                        .pool_xyk()
                        .deposit_liquidity(dex_id, base, asset_id, reserves.0, reserves.1, 1, 1),
                )
                .await?;
            }
        }
        Ok(())
    }
}
