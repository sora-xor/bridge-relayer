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

use bridge_types::{
    ton::{TonAddress, TonNetworkId},
    types::AssetKind,
    MainnetAssetId,
};
use codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_encode::EncodeAsType;
use subxt::utils::Static;

use crate::{
    storage::{StorageEntry, StorageMap},
    tx::SignedTx,
    types::PalletInfo,
    SubResult,
};

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct RegisterNetwork {
    network_id: Static<TonNetworkId>,
    contract: Static<TonAddress>,
    symbol: Vec<u8>,
    name: Vec<u8>,
    decimals: u8,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct RegisterNetworkWithExistingAsset {
    network_id: Static<TonNetworkId>,
    contract: Static<TonAddress>,
    asset_id: MainnetAssetId,
    decimals: u8,
}

const PALLET: PalletInfo = PalletInfo::new("JettonApp");

const REGISTER_NETWORK_CALL: SignedTx<RegisterNetwork> = SignedTx::new(PALLET, "register_network");
const REGISTER_NETWORK_WITH_EXISTING_ASSET_CALL: SignedTx<RegisterNetworkWithExistingAsset> =
    SignedTx::new(PALLET, "register_network_with_existing_asset");

const APP_INFO: StorageEntry<(TonNetworkId, TonAddress), ()> = StorageEntry::new(PALLET, "AppInfo");
const ASSET_KINDS: StorageMap<MainnetAssetId, AssetKind, ()> =
    StorageMap::new(PALLET, "AssetKinds");
const ASSETS_BY_ADDRESSES: StorageMap<TonAddress, MainnetAssetId, ()> =
    StorageMap::new(PALLET, "AssetsByAddresses");
const TOKEN_ADDRESSES: StorageMap<MainnetAssetId, TonAddress, ()> =
    StorageMap::new(PALLET, "TokenAddresses");

#[async_trait::async_trait]
pub trait TonAppTx<T: subxt::Config> {
    async fn register_network(
        &self,
        network_id: TonNetworkId,
        contract: TonAddress,
        symbol: Vec<u8>,
        name: Vec<u8>,
        decimals: u8,
    ) -> SubResult<()>;

    async fn register_network_with_existing_asset(
        &self,
        network_id: TonNetworkId,
        contract: TonAddress,
        asset_id: MainnetAssetId,
        decimals: u8,
    ) -> SubResult<()>;
}

#[async_trait::async_trait]
pub trait TonAppStorage<T: subxt::Config> {
    async fn app_info(&self) -> SubResult<Option<(TonNetworkId, TonAddress)>>;
    async fn asset_kind(&self, asset_id: MainnetAssetId) -> SubResult<Option<AssetKind>>;
    async fn asset_by_address(&self, address: TonAddress) -> SubResult<Option<MainnetAssetId>>;
    async fn address_by_asset(&self, asset_id: MainnetAssetId) -> SubResult<Option<TonAddress>>;
}

#[async_trait::async_trait]
impl<T: subxt::Config> TonAppStorage<T> for crate::Storages<T> {
    async fn app_info(&self) -> SubResult<Option<(TonNetworkId, TonAddress)>> {
        APP_INFO.fetch(self).await
    }

    async fn asset_kind(&self, asset_id: MainnetAssetId) -> SubResult<Option<AssetKind>> {
        ASSET_KINDS.fetch(self, asset_id).await
    }

    async fn asset_by_address(&self, address: TonAddress) -> SubResult<Option<MainnetAssetId>> {
        ASSETS_BY_ADDRESSES.fetch(self, address).await
    }

    async fn address_by_asset(&self, asset_id: MainnetAssetId) -> SubResult<Option<TonAddress>> {
        TOKEN_ADDRESSES.fetch(self, asset_id).await
    }
}

#[async_trait::async_trait]
impl<T, P> TonAppTx<T> for crate::tx::SignedTxs<T, P>
where
    T: subxt::Config,
    P: sp_core::Pair + Send + Sync + Clone,
    <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default + Send + Sync,
    T::Signature: From<P::Signature> + Send + Sync,
    T::AccountId: From<sp_runtime::AccountId32> + Send + Sync,
{
    async fn register_network(
        &self,
        network_id: TonNetworkId,
        contract: TonAddress,
        symbol: Vec<u8>,
        name: Vec<u8>,
        decimals: u8,
    ) -> SubResult<()> {
        REGISTER_NETWORK_CALL
            .submit_sudo(
                self,
                RegisterNetwork {
                    network_id: Static(network_id),
                    contract: Static(contract),
                    symbol,
                    name,
                    decimals,
                },
            )
            .await
    }

    async fn register_network_with_existing_asset(
        &self,
        network_id: TonNetworkId,
        contract: TonAddress,
        asset_id: MainnetAssetId,
        decimals: u8,
    ) -> SubResult<()> {
        REGISTER_NETWORK_WITH_EXISTING_ASSET_CALL
            .submit_sudo(
                self,
                RegisterNetworkWithExistingAsset {
                    network_id: Static(network_id),
                    contract: Static(contract),
                    asset_id,
                    decimals,
                },
            )
            .await
    }
}
