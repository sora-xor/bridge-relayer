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

use std::marker::PhantomData;

use codec::{Decode, Encode};
use subxt::{
    storage::{StaticAddress, StaticStorageKey},
    utils::{Static, Yes},
};

use crate::{types::PalletInfo, Error, SubResult, UnsignedClient};

pub struct Storages<T: subxt::Config> {
    client: UnsignedClient<T>,
    storage: subxt::storage::Storage<T, subxt::OnlineClient<T>>,
}

impl<T: subxt::Config> Storages<T> {
    pub async fn from_client(client: UnsignedClient<T>) -> SubResult<Self> {
        if let Some(at) = client.at {
            Ok(Self {
                storage: client.inner.storage().at(at),
                client,
            })
        } else {
            Ok(Self {
                storage: client.inner.storage().at_latest().await?,
                client,
            })
        }
    }

    pub fn client(&self) -> UnsignedClient<T> {
        self.client.clone()
    }

    pub fn storage(&self) -> subxt::storage::Storage<T, subxt::OnlineClient<T>> {
        self.storage.clone()
    }

    pub fn is_supported(&self, pallet: &str, entry: &str) -> bool {
        let metadata = self.client().metadata();
        metadata
            .pallet_by_name(pallet)
            .and_then(|p| p.storage())
            .and_then(|s| s.entry_by_name(entry))
            .is_some()
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct StorageEntry<R, Defaultable = Yes> {
    pub pallet: PalletInfo,
    pub entry: &'static str,
    #[derive_where(skip)]
    _phantom: PhantomData<(R, Defaultable)>,
}

impl<R, Defaultable> StorageEntry<R, Defaultable>
where
    R: Decode,
{
    pub const fn new(pallet: PalletInfo, entry: &'static str) -> Self {
        Self {
            pallet,
            entry,
            _phantom: PhantomData,
        }
    }

    pub fn address(&self) -> StaticAddress<(), Static<R>, Yes, Defaultable, ()> {
        StaticAddress::new_static(self.pallet.name, self.entry, (), [0u8; 32]).unvalidated()
    }

    #[instrument(skip(storage))]
    pub async fn fetch<T: subxt::Config>(&self, storage: &Storages<T>) -> SubResult<Option<R>> {
        if self.is_supported(storage) {
            Ok(storage.storage().fetch(&self.address()).await?.map(|v| v.0))
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }

    pub fn is_supported<T: subxt::Config>(&self, storage: &Storages<T>) -> bool {
        storage.is_supported(self.pallet.name, self.entry)
    }

    pub fn prefix(&self) -> [u8; 32] {
        let mut prefix = [0u8; 32];
        prefix[..16].copy_from_slice(&self.pallet.prefix());
        prefix[16..].copy_from_slice(&sp_core::twox_128(self.entry.as_bytes()));
        prefix
    }
}

impl<R> StorageEntry<R, Yes>
where
    R: Decode,
{
    #[instrument(skip(storage))]
    pub async fn fetch_or_default<T: subxt::Config>(&self, storage: &Storages<T>) -> SubResult<R> {
        if self.is_supported(storage) {
            Ok(storage.storage().fetch_or_default(&self.address()).await?.0)
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct StorageMap<K, R, Defaultable = Yes> {
    pub pallet: PalletInfo,
    pub entry: &'static str,
    #[derive_where(skip)]
    _phantom: PhantomData<(K, R, Defaultable)>,
}

impl<K, R, Defaultable> StorageMap<K, R, Defaultable>
where
    K: Encode + std::fmt::Debug,
    R: Decode,
{
    pub const fn new(pallet: PalletInfo, entry: &'static str) -> Self {
        Self {
            pallet,
            entry,
            _phantom: PhantomData,
        }
    }

    pub fn address(
        &self,
        key: K,
    ) -> StaticAddress<StaticStorageKey<K>, Static<R>, Yes, Defaultable, Yes> {
        StaticAddress::new_static(
            self.pallet.name,
            self.entry,
            StaticStorageKey::new(&key),
            [0u8; 32],
        )
        .unvalidated()
    }

    #[instrument(skip(storage))]
    pub async fn fetch<T: subxt::Config>(
        &self,
        storage: &Storages<T>,
        key: K,
    ) -> SubResult<Option<R>> {
        if self.is_supported(storage) {
            Ok(storage
                .storage()
                .fetch(&self.address(key))
                .await?
                .map(|v| v.0))
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }

    pub fn is_supported<T: subxt::Config>(&self, storage: &Storages<T>) -> bool {
        storage.is_supported(self.pallet.name, self.entry)
    }

    pub fn prefix(&self) -> [u8; 32] {
        let mut prefix = [0u8; 32];
        prefix[..16].copy_from_slice(&self.pallet.prefix());
        prefix[16..].copy_from_slice(&sp_core::twox_128(self.entry.as_bytes()));
        prefix
    }
}

impl<K, R> StorageMap<K, R, Yes>
where
    K: Encode + std::fmt::Debug,
    R: Decode,
{
    #[instrument(skip(storage))]
    pub async fn fetch_or_default<T: subxt::Config>(
        &self,
        storage: &Storages<T>,
        key: K,
    ) -> SubResult<R> {
        if self.is_supported(storage) {
            Ok(storage
                .storage()
                .fetch_or_default(&self.address(key))
                .await?
                .0)
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct StorageDoubleMap<K1, K2, R, Defaultable = Yes> {
    pub pallet: PalletInfo,
    pub entry: &'static str,
    #[derive_where(skip)]
    _phantom: PhantomData<(K1, K2, R, Defaultable)>,
}

impl<K1, K2, R, Defaultable> StorageDoubleMap<K1, K2, R, Defaultable>
where
    K1: Encode + std::fmt::Debug,
    K2: Encode + std::fmt::Debug,
    R: Decode,
{
    pub const fn new(pallet: PalletInfo, entry: &'static str) -> Self {
        Self {
            pallet,
            entry,
            _phantom: PhantomData,
        }
    }

    pub fn address(
        &self,
        key1: K1,
        key2: K2,
    ) -> StaticAddress<(StaticStorageKey<K1>, StaticStorageKey<K2>), Static<R>, Yes, Defaultable, Yes>
    {
        StaticAddress::new_static(
            self.pallet.name,
            self.entry,
            (StaticStorageKey::new(&key1), StaticStorageKey::new(&key2)),
            [0u8; 32],
        )
        .unvalidated()
    }

    #[instrument(skip(storage))]
    pub async fn fetch<T: subxt::Config>(
        &self,
        storage: &crate::Storages<T>,
        key1: K1,
        key2: K2,
    ) -> SubResult<Option<R>> {
        if self.is_supported(storage) {
            Ok(storage
                .storage()
                .fetch(&self.address(key1, key2))
                .await?
                .map(|v| v.0))
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }

    pub fn is_supported<T: subxt::Config>(&self, storage: &Storages<T>) -> bool {
        storage.is_supported(self.pallet.name, self.entry)
    }

    pub fn prefix(&self) -> [u8; 32] {
        let mut prefix = [0u8; 32];
        prefix[..16].copy_from_slice(&self.pallet.prefix());
        prefix[16..].copy_from_slice(&sp_core::twox_128(self.entry.as_bytes()));
        prefix
    }
}

impl<K1, K2, R> StorageDoubleMap<K1, K2, R, Yes>
where
    K1: Encode + std::fmt::Debug,
    K2: Encode + std::fmt::Debug,
    R: Decode,
{
    #[instrument(skip(storage))]
    pub async fn fetch_or_default<T: subxt::Config>(
        &self,
        storage: &crate::Storages<T>,
        key1: K1,
        key2: K2,
    ) -> SubResult<R> {
        if self.is_supported(storage) {
            Ok(storage
                .storage()
                .fetch_or_default(&self.address(key1, key2))
                .await?
                .0)
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }
}
