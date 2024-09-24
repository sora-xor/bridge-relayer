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

#[macro_use]
extern crate tracing;

pub mod abi;
pub mod config;
pub mod constant;
pub mod error;
pub mod metrics;
pub mod signer;
pub mod storage;
pub mod tx;
pub mod types;
pub mod unsigned_tx;

pub type BlockNumberOf<T> = <<T as subxt::Config>::Header as subxt::config::Header>::Number;

use ::metrics::Label;
use async_lock::RwLock;
use jsonrpsee::client_transport::ws::WsTransportClientBuilder;
use metrics::RpcClientMetrics;
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
    nonce: Arc<RwLock<Option<u64>>>,
    at: Option<T::Hash>,
    labels: Vec<Label>,
}

impl<T: subxt::Config, S: Clone> Clone for Client<T, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            rpc: self.rpc.clone(),
            metadata: self.metadata.clone(),
            signer: self.signer.clone(),
            at: self.at,
            nonce: self.nonce.clone(),
            labels: self.labels.clone(),
        }
    }
}

pub type SignedClient<T, P> = Client<T, signer::Signed<P>>;

pub type UnsignedClient<T> = Client<T, signer::Unsigned>;

impl<T: subxt::Config> UnsignedClient<T> {
    pub async fn from_url(url: &str, labels: Vec<Label>) -> SubResult<Self> {
        let (sender, receiver) = WsTransportClientBuilder::default()
            .build(url.parse()?)
            .await?;
        let rpc = jsonrpsee::core::client::Client::builder()
            .max_buffer_capacity_per_subscription(4096)
            .build_with_tokio(sender, receiver);
        let rpc = subxt::backend::rpc::RpcClient::new(RpcClientMetrics(rpc, labels.clone()));
        let client = subxt::OnlineClient::from_rpc_client(rpc.clone()).await?;
        Ok(Self {
            inner: client,
            rpc,
            signer: signer::Unsigned,
            metadata: Default::default(),
            at: None,
            nonce: Default::default(),
            labels,
        })
    }

    pub fn labels(&self) -> Vec<Label> {
        self.labels.clone()
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub async fn follow_runtime_upgrades(&self) -> SubResult<()> {
        Ok(self.inner.updater().perform_runtime_updates().await?)
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
            at: self.at,
            nonce: self.nonce.clone(),
            labels: self.labels.clone(),
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
            at: self.at,
            nonce: self.nonce.clone(),
            labels: self.labels.clone(),
        }
    }

    pub fn metadata(&self) -> subxt::Metadata {
        self.inner.metadata()
    }

    pub async fn storage(&self) -> SubResult<Storages<T>> {
        Storages::from_client(self.unsigned()).await
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
                nonce: self.nonce.clone(),
                labels: self.labels.clone(),
            })
        } else {
            let mut client = self.clone();
            client.at = Some(block.hash());
            Ok(client)
        }
    }

    pub async fn block_ref<N: Into<BlockNumberOrHash>>(
        &self,
        at: N,
    ) -> SubResult<subxt::blocks::BlockRef<T::Hash>> {
        let at = at.into();
        let block_hash = match at {
            BlockNumberOrHash::Number(n) => self
                .methods()
                .chain_get_block_hash(Some(n.into()))
                .await?
                .ok_or(Error::BlockNotFound(at))?,
            BlockNumberOrHash::Hash(h) => T::Hash::decode(&mut &h[..])?,
            BlockNumberOrHash::Best => self
                .methods()
                .chain_get_block_hash(None)
                .await?
                .ok_or(Error::BlockNotFound(at))?,
            BlockNumberOrHash::Finalized => self.methods().chain_get_finalized_head().await?,
        };
        Ok(block_hash.into())
    }

    fn log_events(events: subxt::blocks::ExtrinsicEvents<T>) -> SubResult<()> {
        for event in events.iter() {
            let event = event?;
            debug!(
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
            let status = status?;
            match &status {
                TxStatus::Validated
                | TxStatus::NoLongerInBestBlock
                | TxStatus::Broadcasted { num_peers: _ } => {
                    trace!("Skip status: {status:?}");
                }
                TxStatus::Error { message } => return Err(Error::TxSubmit(message.clone())),
                TxStatus::Invalid { message } => return Err(Error::TxInvalid(message.clone())),
                TxStatus::Dropped { message } => return Err(Error::TxDropped(message.clone())),
                TxStatus::InBestBlock(tx) => {
                    tracing::info!("Tx in block: {:?}", tx.block_hash());
                }
                TxStatus::InFinalizedBlock(tx) => {
                    let events = tx.wait_for_success().await?;
                    tracing::info!("Tx finalized: {:?}", tx.block_hash());
                    Self::log_events(events)?;
                    return Ok(());
                }
            }
        }
    }

    pub async fn unsigned_tx(&self) -> SubResult<unsigned_tx::UnsignedTxs<T>> {
        unsigned_tx::UnsignedTxs::from_client(self.unsigned()).await
    }
}

impl<T, P> SignedClient<T, P>
where
    T: subxt::Config,
    P: sp_core::Pair + Send + Sync + Clone + 'static,
    P::Signature: Into<T::Signature>,
    T::AccountId: From<sp_runtime::AccountId32>,
{
    pub fn signer(&self) -> &signer::Signed<P> {
        &self.signer
    }

    pub async fn tx(&self) -> SubResult<tx::SignedTxs<T, P>> {
        tx::SignedTxs::from_client(self.clone()).await
    }

    #[instrument(skip(self), ret(level = "debug"))]
    pub async fn nonce(&self) -> SubResult<u64> {
        let mut nonce = self.nonce.write_arc().await;
        if let Some(nonce) = nonce.as_mut() {
            *nonce += 1;
            Ok(*nonce)
        } else {
            let new_nonce = self
                .inner
                .blocks()
                .at(self.block_ref(BlockNumberOrHash::Best).await?)
                .await?
                .account_nonce(&subxt::tx::Signer::<T>::account_id(self.signer()))
                .await?;
            *nonce = Some(new_nonce);
            Ok(new_nonce)
        }
    }
}
