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

pub mod abi;
pub mod config;
pub mod constant;
pub mod error;
pub mod signer;
pub mod storage;
pub mod tx;
pub mod types;
pub mod unsigned_tx;

pub type BlockNumberOf<T> = <<T as subxt::Config>::Header as subxt::config::Header>::Number;

use async_lock::RwLock;
use std::{collections::BTreeMap, sync::Arc};

pub use crate::error::{Error, SubResult};
pub use constant::Constants;
pub use storage::Storages;
pub use subxt::tx::Payload as TxPayload;
pub use tx::SignedTxs;
pub use unsigned_tx::UnsignedTxs;

pub use bridge_types;
pub use sp_core;
pub use sp_runtime;

use codec::Decode;
use subxt::{backend::BackendExt, client::RuntimeVersion, OnlineClient};

use self::types::BlockNumberOrHash;

pub struct Client<T: subxt::Config, S> {
    inner: subxt::OnlineClient<T>,
    rpc: subxt::backend::rpc::RpcClient,
    metadata: Arc<RwLock<BTreeMap<u32, subxt::Metadata>>>,
    signer: S,
    at: Option<T::Hash>,
}

impl<T: subxt::Config, S: Clone> Clone for Client<T, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            rpc: self.rpc.clone(),
            metadata: self.metadata.clone(),
            signer: self.signer.clone(),
            at: self.at.clone(),
        }
    }
}

pub type SignedClient<T, P> = Client<T, signer::Signed<P>>;

pub type UnsignedClient<T> = Client<T, signer::Unsigned>;

impl<T: subxt::Config> UnsignedClient<T> {
    pub async fn from_url(url: &str) -> SubResult<Self> {
        let rpc = subxt::backend::rpc::RpcClient::from_url(url).await?;
        let client = subxt::OnlineClient::from_rpc_client(rpc.clone()).await?;
        Ok(Self {
            inner: client,
            rpc,
            signer: signer::Unsigned,
            metadata: Default::default(),
            at: None,
        })
    }

    pub fn signed<P>(&self, pair: P) -> SignedClient<T, P>
    where
        P: sp_core::Pair + Send + Sync + 'static,
        <sp_runtime::MultiSignature as sp_runtime::traits::Verify>::Signer: From<P::Public>,
    {
        SignedClient {
            inner: self.inner.clone(),
            rpc: self.rpc.clone(),
            metadata: self.metadata.clone(),
            signer: signer::Signed::new(pair),
            at: self.at.clone(),
        }
    }
}

impl<T: subxt::Config, S: Clone + Sync + Send + 'static> Client<T, S> {
    pub fn methods(&self) -> subxt::backend::legacy::LegacyRpcMethods<T> {
        subxt::backend::legacy::LegacyRpcMethods::new(self.rpc.clone())
    }

    pub fn unsigned(&self) -> UnsignedClient<T> {
        UnsignedClient {
            inner: self.inner.clone(),
            rpc: self.rpc.clone(),
            metadata: self.metadata.clone(),
            signer: signer::Unsigned,
            at: self.at.clone(),
        }
    }

    pub fn metadata(&self) -> subxt::Metadata {
        self.inner.metadata()
    }

    pub async fn storage(&self) -> SubResult<Storages<T>> {
        Ok(Storages::from_client(self.unsigned()).await?)
    }

    pub fn constants(&self) -> Constants<T> {
        Constants::from_client(self.unsigned())
    }

    async fn metadata_at(
        &self,
        version: RuntimeVersion,
        block: T::Hash,
    ) -> SubResult<subxt::Metadata> {
        if let Some(metadata) = self.metadata.read_arc().await.get(&version.spec_version) {
            Ok(metadata.clone())
        } else {
            let metadata = self.inner.backend().legacy_metadata(block).await?;
            self.metadata
                .write_arc()
                .await
                .insert(version.spec_version, metadata.clone());
            Ok(metadata)
        }
    }

    pub async fn at<N: Into<BlockNumberOrHash>>(&self, at: N) -> SubResult<Self> {
        let block = self.block_ref(at).await?;
        let runtime_version = self
            .methods()
            .state_get_runtime_version(Some(block.hash()))
            .await?;
        let runtime_version = RuntimeVersion {
            spec_version: runtime_version.spec_version,
            transaction_version: runtime_version.transaction_version,
        };
        let genesis_hash = self.inner.genesis_hash();
        if self.inner.runtime_version() != runtime_version {
            let metadata = self.metadata_at(runtime_version, block.hash()).await?;
            let client = OnlineClient::from_rpc_client_with(
                genesis_hash,
                runtime_version,
                metadata,
                self.rpc.clone(),
            )?;
            Ok(Self {
                inner: client,
                rpc: self.rpc.clone(),
                metadata: self.metadata.clone(),
                signer: self.signer.clone(),
                at: Some(block.hash()),
            })
        } else {
            Ok(self.clone())
        }
    }

    pub async fn block_ref<N: Into<BlockNumberOrHash>>(
        &self,
        at: N,
    ) -> SubResult<subxt::blocks::BlockRef<T::Hash>> {
        let at = at.into();
        let block_hash = match at {
            BlockNumberOrHash::Number(n) => {
                let hash = self
                    .methods()
                    .chain_get_block_hash(Some(n.into()))
                    .await?
                    .ok_or(Error::BlockNotFound(at))?;
                hash
            }
            BlockNumberOrHash::Hash(h) => T::Hash::decode(&mut &h[..])?,
            BlockNumberOrHash::Best => {
                let hash = self
                    .methods()
                    .chain_get_block_hash(None)
                    .await?
                    .ok_or(Error::BlockNotFound(at))?;
                hash
            }
            BlockNumberOrHash::Finalized => {
                let hash = self.methods().chain_get_finalized_head().await?;
                hash
            }
        };
        Ok(block_hash.into())
    }

    fn log_events(events: subxt::blocks::ExtrinsicEvents<T>) -> SubResult<()> {
        for event in events.iter() {
            let event = event?;
            log::debug!(
                "{}::{}({})",
                event.pallet_name(),
                event.variant_name(),
                event.field_values()?
            );
        }
        Ok(())
    }

    pub async fn wait_for_success(
        &self,
        mut progress: subxt::tx::TxProgress<T, subxt::OnlineClient<T>>,
    ) -> SubResult<()> {
        use subxt::tx::TxStatus;
        loop {
            let Some(status) = progress.next().await else {
                return Err(Error::TxStatusMissing);
            };
            match status? {
                TxStatus::Validated
                | TxStatus::NoLongerInBestBlock
                | TxStatus::Broadcasted { num_peers: _ } => {}
                TxStatus::Error { message } => return Err(Error::TxSubmit(message)),
                TxStatus::Invalid { message } => return Err(Error::TxInvalid(message)),
                TxStatus::Dropped { message } => return Err(Error::TxDropped(message)),
                TxStatus::InBestBlock(tx) => {
                    let events = tx.wait_for_success().await?;
                    log::info!("Tx in block: {:?}", tx.block_hash());
                    Self::log_events(events)?;
                    return Ok(());
                }
                TxStatus::InFinalizedBlock(tx) => {
                    let events = tx.wait_for_success().await?;
                    log::info!("Tx in block: {:?}", tx.block_hash());
                    Self::log_events(events)?;
                    return Ok(());
                }
            }
        }
    }

    pub async fn submit_unsigned<X: TxPayload>(&self, xt: &X) -> SubResult<()> {
        let progress = self
            .inner
            .tx()
            .create_unsigned(xt)?
            .submit_and_watch()
            .await?;
        self.wait_for_success(progress).await
    }

    pub async fn unsigned_tx(&self) -> SubResult<unsigned_tx::UnsignedTxs<T>> {
        unsigned_tx::UnsignedTxs::from_client(self.unsigned()).await
    }
}

impl<T, P> SignedClient<T, P>
where
    T: subxt::Config,
    P: Send + Sync + Clone + 'static,
{
    pub fn signer(&self) -> &signer::Signed<P> {
        &self.signer
    }

    pub async fn tx(&self) -> SubResult<tx::SignedTxs<T, P>> {
        tx::SignedTxs::from_client(self.clone()).await
    }
}
