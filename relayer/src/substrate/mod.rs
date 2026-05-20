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
pub mod beefy_subscription;
pub mod traits;
pub mod types;

use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::prelude::*;
use bridge_types::types::AuxiliaryDigest;
use bridge_types::GenericNetworkId;
use jsonrpsee::core::client::ClientT;
use sp_core::{ecdsa, Bytes, H256};
use sp_mmr_primitives::{EncodableOpaqueLeaf, Proof};
use sp_runtime::traits::AtLeast32BitUnsigned;
use std::sync::RwLock;
pub use substrate_gen::runtime;
use subxt::blocks::ExtrinsicEvents;
use subxt::constants::ConstantAddress;
use subxt::events::EventDetails;
use subxt::metadata::DecodeWithMetadata;
use subxt::rpc::{ChainBlockResponse, RpcClientT};
use subxt::storage::address::Yes;
use subxt::storage::StorageAddress;
use subxt::tx::Signer;
pub use types::*;

/// Finds the first occurrence of an element 'e' so that 'f(e)' is greater or equal 'value' in
/// storage with ascending values. Returns the index of 'e'.
pub async fn binary_search_first_occurrence<N: AtLeast32BitUnsigned, T: PartialOrd, F, Fut>(
    low: N,
    high: N,
    value: T,
    f: F,
) -> AnyResult<Option<N>>
where
    F: Fn(N) -> Fut,
    Fut: futures::Future<Output = AnyResult<Option<T>>>,
{
    let mut low = low;
    let mut high = high;
    while low < high {
        let mid = (high.clone() + low.clone()) / 2u32.into();
        let found_value = f(mid.clone()).await?;
        match found_value {
            None => low = mid + 1u32.into(),
            Some(found_value) if found_value < value => low = mid + 1u32.into(),
            _ => high = mid,
        }
    }
    // If value between blocks can increase more than by 1
    if f(low.clone()).await? >= Some(value) {
        Ok(Some(low))
    } else {
        Ok(None)
    }
}

pub fn event_to_string<T: ConfigExt>(ev: EventDetails) -> String {
    let input = &mut ev.bytes();
    let phase = subxt::events::Phase::decode(input);
    let event = T::Event::decode(input);
    format!("(Phase: {:?}, Event: {:?})", phase, event)
}

pub fn log_extrinsic_events<T: ConfigExt>(events: ExtrinsicEvents<T::Config>) {
    for ev in events.iter() {
        match ev {
            Ok(ev) => {
                debug!("{}", event_to_string::<T>(ev));
            }
            Err(err) => {
                warn!("Failed to decode event: {:?}", err);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClonableClient(Arc<jsonrpsee::async_client::Client>);

impl RpcClientT for ClonableClient {
    fn request_raw<'a>(
        &'a self,
        method: &'a str,
        params: Option<Box<jsonrpsee::core::JsonRawValue>>,
    ) -> subxt::rpc::RpcFuture<'a, Box<jsonrpsee::core::JsonRawValue>> {
        self.0.request_raw(method, params)
    }

    fn subscribe_raw<'a>(
        &'a self,
        sub: &'a str,
        params: Option<Box<jsonrpsee::core::JsonRawValue>>,
        unsub: &'a str,
    ) -> subxt::rpc::RpcFuture<'a, subxt::rpc::RpcSubscription> {
        self.0.subscribe_raw(sub, params, unsub)
    }
}

#[derive(Debug, Clone)]
pub struct UnsignedClient<T: ConfigExt> {
    api: ApiInner<T>,
    client: ClonableClient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MmrLeavesProof<BlockHash> {
    block_hash: BlockHash,
    leaves: Bytes,
    proof: Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionOutcome {
    Submitted,
    AlreadyInPool,
    TemporarilyBanned,
}

impl<T: ConfigExt> UnsignedClient<T> {
    const POOL_TEMPORARILY_BANNED: i32 = 1012;
    const POOL_ALREADY_IMPORTED: i32 = 1013;
    const POOL_TOO_LOW_PRIORITY: i32 = 1014;

    pub async fn new(url: impl Into<String>) -> AnyResult<Self> {
        let url: Uri = url.into().parse()?;
        let (sender, receiver) =
            jsonrpsee::client_transport::ws::WsTransportClientBuilder::default()
                .build(url)
                .await?;
        let client = jsonrpsee::async_client::ClientBuilder::default()
            .max_notifs_per_subscription(4096)
            .build_with_tokio(sender, receiver);
        let client = ClonableClient(Arc::new(client));
        let api = ApiInner::<T>::from_rpc_client(client.clone().0).await?;
        Ok(Self { api, client })
    }

    pub fn rpc(&self) -> &jsonrpsee::async_client::Client {
        &self.client.0
    }

    pub async fn auxiliary_digest(&self, at: Option<BlockHash<T>>) -> AnyResult<AuxiliaryDigest>
    where
        T: SenderConfig,
    {
        let Some(at) = at else {
            return Err(anyhow::anyhow!(
                "auxiliary_digest: Block Number is incorrect"
            ));
        };
        let latest_digest = T::latest_digest();
        let logs = self.storage_fetch(&latest_digest, at).await?;
        Ok(AuxiliaryDigest {
            logs: logs.unwrap_or_default(),
        })
    }

    pub async fn latest_commitment<N: Into<BlockNumberOrHash>>(
        &self,
        network_id: GenericNetworkId,
        at: N,
    ) -> AnyResult<GenericCommitmentWithBlockOf<T>>
    where
        T: SenderConfig,
    {
        let address = T::latest_commitment(network_id);
        let commitment = self
            .storage_fetch(&address, at)
            .await?
            .ok_or(anyhow!("Commitment not found"))?;
        Ok(commitment)
    }

    pub async fn commitment_with_nonce<N: Into<BlockNumberOrHash>>(
        &self,
        network_id: GenericNetworkId,
        nonce: u64,
        at: N,
    ) -> AnyResult<GenericCommitmentWithBlockOf<T>>
    where
        T: SenderConfig,
    {
        let mut current_block = at.into();
        loop {
            let commitment = self.latest_commitment(network_id, current_block).await?;
            match commitment.commitment.nonce().cmp(&nonce) {
                std::cmp::Ordering::Equal => return Ok(commitment),
                std::cmp::Ordering::Greater => {}
                std::cmp::Ordering::Less => return Err(anyhow!("Commitment nonce too low")),
            }
            current_block = BlockNumberOrHash::Number(
                Into::<u64>::into(commitment.block_number).saturating_sub(1),
            );
        }
    }

    pub async fn beefy_start_block(&self) -> AnyResult<u64> {
        let latest_finalized_hash = self.api().rpc().finalized_head().await?;
        let latest_finalized_number = self
            .api()
            .rpc()
            .block(Some(latest_finalized_hash))
            .await?
            .expect("should exist")
            .block
            .header
            .number()
            .clone();
        let mmr_leaves = self
            .storage_fetch_or_default(&runtime::storage().mmr().number_of_leaves(), ())
            .await?;
        let beefy_start_block = latest_finalized_number.into().saturating_sub(mmr_leaves);
        debug!("Beefy started at: {}", beefy_start_block);
        Ok(beefy_start_block)
    }

    pub async fn mmr_generate_proof(
        &self,
        block_number: BlockNumber<T>,
        at: BlockNumber<T>,
    ) -> AnyResult<LeafProof<T>>
    where
        BlockNumber<T>: Serialize,
    {
        let res = self
            .rpc()
            .request::<MmrLeavesProof<BlockHash<T>>, _>(
                "mmr_generateProof",
                (vec![block_number], Some(at), Option::<BlockHash<T>>::None),
            )
            .await?;

        let enc_opaque_leaf = match Vec::<EncodableOpaqueLeaf>::decode(&mut res.leaves.as_ref()) {
            Ok(mut v) => {
                if v.len() == 0 {
                    error!("Opaque leaves count is zero");
                    Err(anyhow::anyhow!("Opaque leaves count error"))?;
                }
                v.remove(0)
            }
            Err(e) => {
                error!("Error decoding opaque mmr leaves");
                Err(e)?
            }
        };

        let leaf = MmrLeaf::<T>::decode(&mut &*enc_opaque_leaf.into_opaque_leaf().0)?;

        let proof = Proof::<MmrHash>::decode(&mut res.proof.as_ref())?;
        Ok(LeafProof {
            leaf,
            proof,
            block_hash: res.block_hash,
        })
    }

    pub fn api(&self) -> &ApiInner<T> {
        &self.api
    }

    pub async fn header<N: Into<BlockNumberOrHash>>(&self, at: N) -> AnyResult<Header<T>> {
        let hash = self.block_hash(at).await?;
        let header = self
            .api()
            .rpc()
            .header(Some(hash.into()))
            .await?
            .ok_or(anyhow::anyhow!("Header not found"))?;
        Ok(header)
    }

    pub async fn block_number<N: Into<BlockNumberOrHash>>(
        &self,
        at: N,
    ) -> AnyResult<BlockNumber<T>> {
        let header = self.header(at).await?;
        Ok(BlockNumber::<T>::from(header.number().clone()))
    }

    pub async fn finalized_head(&self) -> AnyResult<BlockHash<T>> {
        let hash = self.api().rpc().finalized_head().await?;
        Ok(hash.into())
    }

    pub async fn block_hash<N: Into<BlockNumberOrHash>>(&self, at: N) -> AnyResult<BlockHash<T>> {
        let block_number = match at.into() {
            BlockNumberOrHash::Number(n) => Some(n),
            BlockNumberOrHash::Hash(h) => return Ok(h.into()),
            BlockNumberOrHash::Best => None,
            BlockNumberOrHash::Finalized => {
                return self.finalized_head().await;
            }
        };
        let res = self
            .api()
            .rpc()
            .block_hash(block_number.map(Into::into))
            .await
            .context("Get block hash")?
            .ok_or(anyhow::anyhow!("Block not found"))?;
        Ok(res.into())
    }

    pub async fn block<N: Into<BlockNumberOrHash>>(
        &self,
        at: N,
    ) -> AnyResult<ChainBlockResponse<T::Config>> {
        let hash = self.block_hash(at).await?;
        let block = self
            .api()
            .rpc()
            .block(Some(hash.into()))
            .await?
            .ok_or(anyhow::anyhow!("Block not found"))?;
        Ok(block)
    }

    pub async fn storage_fetch<N, Address>(
        &self,
        address: &Address,
        hash: N,
    ) -> AnyResult<Option<<Address::Target as DecodeWithMetadata>::Target>>
    where
        Address: StorageAddress<IsFetchable = Yes>,
        N: Into<BlockNumberOrHash>,
    {
        let hash = self.block_hash(hash).await?;
        trace!(
            "Fetching storage {}::{} at hash {:?}",
            address.pallet_name(),
            address.entry_name(),
            hash
        );
        let address = Unvalidated(address);
        let res = self
            .api()
            .storage()
            .fetch(&address, Some(hash.into()))
            .await
            .context(format!(
                "Fetch storage {}::{} at hash {:?}",
                address.pallet_name(),
                address.entry_name(),
                hash
            ))?;
        Ok(res)
    }

    pub async fn storage_fetch_or_default<N, Address>(
        &self,
        address: &Address,
        hash: N,
    ) -> AnyResult<<Address::Target as DecodeWithMetadata>::Target>
    where
        Address: StorageAddress<IsFetchable = Yes, IsDefaultable = Yes>,
        N: Into<BlockNumberOrHash>,
    {
        let hash = self.block_hash(hash).await?;
        trace!(
            "Fetching storage {}::{} at hash {:?}",
            address.pallet_name(),
            address.entry_name(),
            hash
        );
        let address = Unvalidated(address);
        let res = self
            .api()
            .storage()
            .fetch_or_default(&address, Some(hash.into()))
            .await
            .context(format!(
                "Fetch storage {}::{} at hash {:?}",
                address.pallet_name(),
                address.entry_name(),
                hash
            ))?;
        Ok(res)
    }

    pub fn constant_fetch_or_default<Address>(
        &self,
        address: &Address,
    ) -> AnyResult<<Address::Target as DecodeWithMetadata>::Target>
    where
        Address: ConstantAddress,
    {
        let address = Unvalidated(address);
        let res = self.api().constants().at(&address)?;
        Ok(res)
    }

    pub async fn signed(self, signer: PairSigner<T>) -> AnyResult<SignedClient<T>> {
        SignedClient::<T>::new(self, signer).await
    }

    pub fn transaction_pool_submission_outcome(error: &subxt::Error) -> Option<SubmissionOutcome> {
        match error {
            subxt::Error::Rpc(subxt::error::RpcError::ClientError(error)) => {
                let Some(error) = error.downcast_ref::<jsonrpsee::core::Error>() else {
                    return None;
                };
                match error {
                    jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Custom(
                        error,
                    )) => match error.code() {
                        Self::POOL_TEMPORARILY_BANNED => Some(SubmissionOutcome::TemporarilyBanned),
                        Self::POOL_ALREADY_IMPORTED | Self::POOL_TOO_LOW_PRIORITY => {
                            Some(SubmissionOutcome::AlreadyInPool)
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn is_transaction_imported_or_banned(error: &subxt::Error) -> bool {
        Self::transaction_pool_submission_outcome(error).is_some()
    }

    pub async fn submit_unsigned_extrinsic<P: subxt::tx::TxPayload>(
        &self,
        xt: &P,
    ) -> AnyResult<()> {
        if let Some(validation) = xt.validation_details() {
            debug!(
                "Submitting extrinsic: {}::{}",
                validation.pallet_name, validation.call_name
            );
        } else {
            debug!("Submitting extrinsic without validation data");
        }
        let xt = Unvalidated(xt);
        let res = self
            .api()
            .tx()
            .create_unsigned(&xt)?
            .submit_and_watch()
            .await
            .map_err(|e| {
                debug!("submit then watch error: {:?}", e);
                e
            })
            .context("sign and submit then watch")?
            .wait_for_in_block()
            .await
            .map_err(|e| {
                debug!("wait for in block error: {:?}", e);
                e
            })
            .context("wait for in block")?
            .wait_for_success()
            .await
            .map_err(|e| {
                debug!("wait for success error: {:?}", e);
                e
            })
            .context("wait for success")?;
        log_extrinsic_events::<T>(res);
        Ok(())
    }

    pub async fn submit_concurrent_unsigned_extrinsic<P: subxt::tx::TxPayload>(
        &self,
        xt: &P,
    ) -> AnyResult<SubmissionOutcome> {
        let result = self.submit_unsigned_extrinsic(xt).await;
        match result {
            Err(e) => {
                let Some(subxt_error) = e.downcast_ref::<subxt::Error>() else {
                    error!("unexpected error: {:?}", e);
                    return Err(e);
                };
                if let Some(outcome) = Self::transaction_pool_submission_outcome(subxt_error) {
                    Ok(outcome)
                } else {
                    Err(e)
                }
            }
            Ok(()) => Ok(SubmissionOutcome::Submitted),
        }
    }
}

#[derive(Clone)]
pub struct SignedClient<T: ConfigExt> {
    inner: UnsignedClient<T>,
    key: PairSigner<T>,
    nonce: Arc<RwLock<Option<Index<T>>>>,
}

impl<T: ConfigExt> SignedClient<T> {
    pub async fn new(client: UnsignedClient<T>, key: PairSigner<T>) -> AnyResult<Self> {
        let res = Self {
            inner: client,
            key,
            nonce: Arc::new(RwLock::new(None)),
        };
        res.load_nonce().await?;
        Ok(res)
    }

    pub fn account_id(&self) -> AccountId<T> {
        self.key.account_id().clone()
    }

    pub async fn submit_extrinsic<P: subxt::tx::TxPayload>(&self, xt: &P) -> AnyResult<()>
    where
        <<<T as ConfigExt>::Config as subxt::Config>::ExtrinsicParams as subxt::tx::ExtrinsicParams<
            <<T as ConfigExt>::Config as subxt::Config>::Index,
            <<T as ConfigExt>::Config as subxt::Config>::Hash,
        >>::OtherParams: Default,
    {
        if let Some(validation) = xt.validation_details() {
            debug!(
                "Submitting extrinsic: {}::{}",
                validation.pallet_name, validation.call_name
            );
        } else {
            debug!("Submitting extrinsic without validation data");
        }
        // Metadata validation often works incorrectly, so we turn it off for now
        let xt = Unvalidated(xt);
        let res = self
            .api()
            .tx()
            .sign_and_submit_then_watch_default(&xt, self)
            .await
            .map_err(|e| {
                error!("sign and submit then watch error: {:?}", e);
                e
            })
            .context("sign and submit then watch")?
            .wait_for_in_block()
            .await
            .context("wait for in block")?
            .wait_for_success()
            .await
            .context("wait for success")?;
        log_extrinsic_events::<T>(res);
        Ok(())
    }

    pub async fn load_nonce(&self) -> AnyResult<()> {
        let nonce = self
            .inner
            .api()
            .rpc()
            .system_account_next_index(&self.key.account_id())
            .await?;
        self.set_nonce(nonce);
        Ok(())
    }

    pub fn unsigned(self) -> UnsignedClient<T> {
        self.inner
    }

    pub fn api(&self) -> &ApiInner<T> {
        &self.inner.api()
    }

    pub fn set_nonce(&self, index: Index<T>) {
        let mut nonce = self.nonce.write().expect("poisoned");
        *nonce = Some(index);
    }
}

impl<T: ConfigExt> Deref for SignedClient<T> {
    type Target = UnsignedClient<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ConfigExt> DerefMut for SignedClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: ConfigExt> Signer<T::Config> for SignedClient<T> {
    fn account_id(&self) -> &AccountId<T> {
        self.key.account_id()
    }

    fn sign(&self, extrinsic: &[u8]) -> Signature<T> {
        self.key.sign(extrinsic)
    }

    fn address(&self) -> Address<T> {
        self.key.address()
    }
}

impl UnsignedClient<MainnetConfig> {
    pub async fn submit_inbound_commitment(
        &self,
        signer: ecdsa::Pair,
        sender: GenericNetworkId,
        receiver: GenericNetworkId,
        commitment: UnboundedGenericCommitment,
    ) -> AnyResult<()> {
        info!("Submit commitment {commitment:?}");
        let message =
            sp_runtime::traits::Keccak256::hash_of(&(sender, receiver, commitment.hash()));
        self.approve_message(signer, sender, message).await?;
        if self.should_send_commitment(&sender, message).await? {
            info!("Sending commitment");
            let approvals = self.bridge_approvals(&sender, message).await?;
            let proof = VerifierMultiProof::EVMMultisig(
                runtime::runtime_types::multisig_verifier::MultiEVMProof {
                    proof: approvals.try_into().unwrap(),
                },
            );
            let success = self
                .submit_concurrent_unsigned_extrinsic(
                    &runtime::tx()
                        .bridge_inbound_channel()
                        .submit(sender, commitment, proof),
                )
                .await?;
            match success {
                SubmissionOutcome::Submitted => info!("Commitment submitted by this relayer"),
                SubmissionOutcome::AlreadyInPool => {
                    info!("Commitment will be submitted by another relayer")
                }
                SubmissionOutcome::TemporarilyBanned => {
                    warn!("Commitment submission is temporarily banned; retrying after state check")
                }
            }
        }
        Ok(())
    }

    pub async fn approve_message(
        &self,
        signer: ecdsa::Pair,
        sender: GenericNetworkId,
        message: H256,
    ) -> AnyResult<()> {
        if self
            .should_send_approval(&sender, signer.public(), message)
            .await?
        {
            info!("Sending approval");
            let signature = signer.sign_prehashed(&message.0);
            let submitted = self
                .submit_concurrent_unsigned_extrinsic(
                    &runtime::tx()
                        .bridge_data_signer()
                        .approve(sender, message, signature),
                )
                .await?;
            match submitted {
                SubmissionOutcome::Submitted => {}
                SubmissionOutcome::AlreadyInPool => {
                    info!("Approval will be submitted by another relayer or is already in the pool")
                }
                SubmissionOutcome::TemporarilyBanned => {
                    warn!("Approval submission is temporarily banned; retrying after state check")
                }
            }
        }
        Ok(())
    }

    pub async fn should_send_approval(
        &self,
        network_id: &GenericNetworkId,
        signer: ecdsa::Public,
        message: H256,
    ) -> AnyResult<bool> {
        let peers = self.bridge_peers(network_id).await?;
        let approvals = self.bridge_approvals(network_id, message).await?;
        let is_already_approved = approvals
            .iter()
            .filter_map(|approval| approval.recover_prehashed(&message.0))
            .any(|public| signer == public);
        Ok(
            (approvals.len() as u32) < bridge_types::utils::threshold(peers.len() as u32)
                && !is_already_approved,
        )
    }

    pub async fn should_send_commitment(
        &self,
        network_id: &GenericNetworkId,
        message: H256,
    ) -> AnyResult<bool> {
        let peers = self.bridge_peers(network_id).await?;
        let approvals = self.bridge_approvals(network_id, message).await?;
        Ok((approvals.len() as u32) >= bridge_types::utils::threshold(peers.len() as u32))
    }

    pub async fn bridge_approvals(
        &self,
        network_id: &GenericNetworkId,
        message: H256,
    ) -> AnyResult<Vec<ecdsa::Signature>> {
        let peers = self.bridge_peers(network_id).await?;
        let approvals = self
            .storage_fetch_or_default(
                &runtime::storage()
                    .bridge_data_signer()
                    .approvals(network_id, message),
                (),
            )
            .await?;
        let mut acceptable_approvals = vec![];
        for approval in approvals {
            let public = approval
                .1
                .recover_prehashed(&message.0)
                .ok_or(anyhow!("Wrong signature in data signer pallet"))?;
            if peers.contains(&public) {
                acceptable_approvals.push(approval.1);
            }
        }
        Ok(acceptable_approvals)
    }

    pub async fn bridge_peers(
        &self,
        network_id: &GenericNetworkId,
    ) -> AnyResult<BTreeSet<ecdsa::Public>> {
        let peers = self
            .storage_fetch(
                &runtime::storage().multisig_verifier().peer_keys(network_id),
                (),
            )
            .await?
            .unwrap_or_default()
            .into_iter()
            .collect();
        Ok(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::substrate::traits::MainnetConfig;

    type Client = UnsignedClient<MainnetConfig>;

    fn jsonrpsee_call_error(code: i32, message: &str) -> jsonrpsee::core::Error {
        jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Custom(
            jsonrpsee::types::error::ErrorObject::owned(code, message, None::<()>),
        ))
    }

    fn jsonrpsee_call_error_with_data(code: i32, message: &str) -> jsonrpsee::core::Error {
        jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Custom(
            jsonrpsee::types::error::ErrorObject::owned(
                code,
                message,
                Some(serde_json::json!({
                    "error": "pool-like text in data must not change classification",
                    "message": "Transaction is temporarily banned",
                })),
            ),
        ))
    }

    fn jsonrpsee_call_error_with_fake_pool_code_in_data(code: i32) -> jsonrpsee::core::Error {
        jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Custom(
            jsonrpsee::types::error::ErrorObject::owned(
                code,
                "outer error is not a transaction-pool duplicate",
                Some(serde_json::json!({
                    "code": 1012,
                    "message": "Transaction is temporarily banned",
                    "nested": {
                        "code": 1013,
                        "message": "Already Imported",
                    },
                })),
            ),
        ))
    }

    fn subxt_client_error(error: impl std::error::Error + Send + Sync + 'static) -> subxt::Error {
        subxt::Error::Rpc(subxt::error::RpcError::ClientError(Box::new(error)))
    }

    fn subxt_pool_error(code: i32, message: &str) -> subxt::Error {
        subxt_client_error(jsonrpsee_call_error(code, message))
    }

    #[derive(Debug)]
    struct SourceWrappedError(jsonrpsee::core::Error);

    impl std::fmt::Display for SourceWrappedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "wrapped source error: {}", self.0)
        }
    }

    impl std::error::Error for SourceWrappedError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    fn serialized_mmr_proof() -> serde_json::Value {
        serde_json::to_value(MmrLeavesProof {
            block_hash: H256::repeat_byte(1),
            leaves: Bytes(vec![1, 2, 3]),
            proof: Bytes(vec![4, 5, 6]),
        })
        .expect("MMR proof DTO should serialize")
    }

    #[test]
    fn mmr_leaves_proof_rejects_missing_required_fields() {
        for field in ["blockHash", "leaves", "proof"] {
            let mut value = serialized_mmr_proof();
            value
                .as_object_mut()
                .expect("serialized proof should be an object")
                .remove(field);

            assert!(
                serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err(),
                "MMR proof without {field} must fail"
            );
        }
    }

    #[test]
    fn mmr_leaves_proof_rejects_snake_case_block_hash() {
        let mut value = serialized_mmr_proof();
        let object = value
            .as_object_mut()
            .expect("serialized proof should be an object");
        let block_hash = object.remove("blockHash").expect("blockHash should exist");
        object.insert("block_hash".to_string(), block_hash);

        assert!(serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err());
    }

    #[test]
    fn mmr_leaves_proof_rejects_malformed_block_hash() {
        for block_hash in [
            serde_json::json!("0x1234"),
            serde_json::json!("not-a-hash"),
            serde_json::json!(null),
            serde_json::json!(123),
        ] {
            let mut value = serialized_mmr_proof();
            value
                .as_object_mut()
                .expect("serialized proof should be an object")
                .insert("blockHash".to_string(), block_hash);

            assert!(
                serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err(),
                "MMR proof with malformed block hash must fail"
            );
        }
    }

    #[test]
    fn mmr_leaves_proof_rejects_malformed_byte_fields() {
        for field in ["leaves", "proof"] {
            let mut value = serialized_mmr_proof();
            value
                .as_object_mut()
                .expect("serialized proof should be an object")
                .insert(field.to_string(), serde_json::json!(123));

            assert!(
                serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err(),
                "MMR proof with malformed {field} must fail"
            );
        }
    }

    #[test]
    fn mmr_leaves_proof_rejects_null_byte_fields() {
        for field in ["leaves", "proof"] {
            let mut value = serialized_mmr_proof();
            value
                .as_object_mut()
                .expect("serialized proof should be an object")
                .insert(field.to_string(), serde_json::Value::Null);

            assert!(
                serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err(),
                "MMR proof with null {field} must fail"
            );
        }
    }

    #[test]
    fn mmr_leaves_proof_rejects_non_object_json() {
        for value in [
            serde_json::json!(null),
            serde_json::json!([]),
            serde_json::json!("not-an-object"),
        ] {
            assert!(
                serde_json::from_value::<MmrLeavesProof<H256>>(value).is_err(),
                "MMR proof must be an object"
            );
        }
    }

    #[test]
    fn recognizes_duplicate_or_banned_pool_codes() {
        for (code, message, outcome) in [
            (
                1012,
                "Transaction is temporarily banned",
                SubmissionOutcome::TemporarilyBanned,
            ),
            (1013, "Already Imported", SubmissionOutcome::AlreadyInPool),
            (
                1014,
                "The transaction has too low priority to replace another transaction already in the pool.",
                SubmissionOutcome::AlreadyInPool,
            ),
        ] {
            let error = subxt_pool_error(code, message);
            assert_eq!(
                Client::transaction_pool_submission_outcome(&error),
                Some(outcome),
                "expected pool code {code} to classify to {outcome:?}"
            );
            assert!(
                Client::is_transaction_imported_or_banned(&error),
                "expected pool code {code} to be tolerated"
            );
        }
    }

    #[test]
    fn recognizes_pool_codes_with_unexpected_messages_and_data() {
        for code in [1012, 1013, 1014] {
            let error = subxt_client_error(jsonrpsee_call_error_with_data(
                code,
                "unexpected upstream message",
            ));
            assert!(
                Client::is_transaction_imported_or_banned(&error),
                "expected pool code {code} to be tolerated independent of message/data"
            );
        }
    }

    #[test]
    fn rejects_spoofed_messages_with_unrecognized_codes() {
        for (code, message) in [
            (1010, "Transaction is temporarily banned"),
            (1011, "Transaction is temporarily banned"),
            (1015, "Already Imported"),
            (1016, "The transaction has too low priority to replace another transaction already in the pool."),
            (9999, "Transaction is temporarily banned"),
            (-32700, "Transaction is temporarily banned"),
            (-32603, "Already Imported"),
            (-32604, "The transaction has too low priority to replace another transaction already in the pool."),
            (-32000, "Transaction is temporarily banned"),
        ] {
            let error = subxt_pool_error(code, message);
            assert!(
                Client::transaction_pool_submission_outcome(&error).is_none(),
                "unexpectedly tolerated spoofed pool message with code {code}"
            );
        }
    }

    #[test]
    fn rejects_near_miss_pool_codes_around_the_allowed_range() {
        for code in 1008..=1018 {
            if [1012, 1013, 1014].contains(&code) {
                continue;
            }

            for message in [
                "Transaction is temporarily banned",
                "Already Imported",
                "The transaction has too low priority to replace another transaction already in the pool.",
            ] {
                let error = subxt_pool_error(code, message);
                assert_eq!(
                    Client::transaction_pool_submission_outcome(&error),
                    None,
                    "unexpectedly tolerated near-miss pool code {code}"
                );
            }
        }
    }

    #[test]
    fn rejects_pool_codes_hidden_in_jsonrpc_data() {
        for code in [-32700, -32603, -32000, 0, 1011, 1015] {
            let error = subxt_client_error(jsonrpsee_call_error_with_fake_pool_code_in_data(code));
            assert!(
                !Client::is_transaction_imported_or_banned(&error),
                "unexpectedly tolerated fake nested pool code under outer code {code}"
            );
        }
    }

    #[test]
    fn rejects_extreme_codes_even_with_pool_like_messages() {
        for code in [i32::MIN, -1, 0, 1, 1000, 10_120, i32::MAX] {
            for message in [
                "Transaction is temporarily banned",
                "Already Imported",
                "The transaction has too low priority to replace another transaction already in the pool.",
            ] {
                let error = subxt_pool_error(code, message);
                assert!(
                    !Client::is_transaction_imported_or_banned(&error),
                    "unexpectedly tolerated extreme or unrelated code {code}"
                );
            }
        }
    }

    #[test]
    fn rejects_non_custom_jsonrpsee_call_errors() {
        let failed_call = jsonrpsee::core::Error::Call(jsonrpsee::types::error::CallError::Failed(
            anyhow!("Transaction is temporarily banned"),
        ));
        let invalid_params = jsonrpsee::core::Error::Call(
            jsonrpsee::types::error::CallError::InvalidParams(anyhow!("Already Imported")),
        );

        for error in [
            subxt_client_error(failed_call),
            subxt_client_error(invalid_params),
        ] {
            assert!(!Client::is_transaction_imported_or_banned(&error));
        }
    }

    #[test]
    fn rejects_jsonrpsee_non_call_errors_with_pool_like_text() {
        for error in [
            jsonrpsee::core::Error::Custom("Transaction is temporarily banned".to_string()),
            jsonrpsee::core::Error::Custom("Already Imported".to_string()),
            jsonrpsee::core::Error::RequestTimeout,
            jsonrpsee::core::Error::MethodNotFound("author_submitAndWatchExtrinsic".to_string()),
            jsonrpsee::core::Error::InvalidSubscriptionId,
        ] {
            assert!(!Client::is_transaction_imported_or_banned(
                &subxt_client_error(error)
            ));
        }
    }

    #[test]
    fn rejects_additional_jsonrpsee_non_pool_variants() {
        for error in [
            jsonrpsee::core::Error::Transport(anyhow!("Transaction is temporarily banned")),
            jsonrpsee::core::Error::RestartNeeded("Already Imported".to_string()),
            jsonrpsee::core::Error::InvalidRequestId,
            jsonrpsee::core::Error::DuplicateRequestId,
            jsonrpsee::core::Error::MethodAlreadyRegistered(
                "author_submitAndWatchExtrinsic".to_string(),
            ),
            jsonrpsee::core::Error::SubscriptionNameConflict(
                "author_submitAndWatchExtrinsic".to_string(),
            ),
            jsonrpsee::core::Error::MaxSlotsExceeded,
            jsonrpsee::core::Error::HttpNotImplemented,
            jsonrpsee::core::Error::EmptyBatchRequest,
        ] {
            assert!(!Client::is_transaction_imported_or_banned(
                &subxt_client_error(error)
            ));
        }
    }

    #[test]
    fn rejects_non_jsonrpsee_client_errors_with_pool_like_text() {
        let io_error = std::io::Error::new(
            std::io::ErrorKind::Other,
            "Transaction is temporarily banned",
        );
        let error = subxt_client_error(io_error);

        assert!(!Client::is_transaction_imported_or_banned(&error));
    }

    #[test]
    fn rejects_source_wrapped_jsonrpsee_client_errors() {
        let wrapped = SourceWrappedError(jsonrpsee_call_error(
            1012,
            "Transaction is temporarily banned",
        ));
        let error = subxt_client_error(wrapped);

        assert!(!Client::is_transaction_imported_or_banned(&error));
    }

    #[test]
    fn rejects_pool_codes_encoded_only_in_plaintext_client_errors() {
        for message in [
            "Custom error: code=1012 message='Transaction is temporarily banned'",
            "JSON-RPC error 1013: Already Imported",
            "1014: The transaction has too low priority to replace another transaction already in the pool.",
            r#"{"code":1012,"message":"Transaction is temporarily banned"}"#,
            r#"{"error":{"code":1013,"message":"Already Imported"}}"#,
        ] {
            let error = subxt_client_error(std::io::Error::new(std::io::ErrorKind::Other, message));

            assert_eq!(Client::transaction_pool_submission_outcome(&error), None);
        }
    }

    #[test]
    fn rejects_pool_codes_when_outer_jsonrpc_code_is_server_error() {
        for outer_code in [-32099, -32000, -32603] {
            for hidden_code in [1012, 1013, 1014] {
                let error = subxt_client_error(jsonrpsee::core::Error::Call(
                    jsonrpsee::types::error::CallError::Custom(
                        jsonrpsee::types::error::ErrorObject::owned(
                            outer_code,
                            "server error with misleading pool details",
                            Some(serde_json::json!({
                                "code": hidden_code,
                                "message": "Already Imported",
                            })),
                        ),
                    ),
                ));

                assert_eq!(
                    Client::transaction_pool_submission_outcome(&error),
                    None,
                    "unexpectedly trusted hidden pool code {hidden_code} under outer code {outer_code}"
                );
            }
        }
    }

    #[test]
    fn rejects_pool_codes_hidden_in_stringified_jsonrpc_data() {
        for outer_code in [-32099, -32000, -32603, 1011, 1015] {
            let error = subxt_client_error(jsonrpsee::core::Error::Call(
                jsonrpsee::types::error::CallError::Custom(
                    jsonrpsee::types::error::ErrorObject::owned(
                        outer_code,
                        "outer code must be authoritative",
                        Some(r#"{"code":1012,"message":"Transaction is temporarily banned"}"#),
                    ),
                ),
            ));

            assert_eq!(
                Client::transaction_pool_submission_outcome(&error),
                None,
                "unexpectedly trusted stringified JSON data under outer code {outer_code}"
            );
        }
    }

    #[test]
    fn rejects_pool_codes_hidden_in_array_jsonrpc_data() {
        for outer_code in [-32000, 0, 1011, 1015] {
            let error = subxt_client_error(jsonrpsee::core::Error::Call(
                jsonrpsee::types::error::CallError::Custom(
                    jsonrpsee::types::error::ErrorObject::owned(
                        outer_code,
                        "outer code must be authoritative",
                        Some(serde_json::json!([
                            {"code": 1012, "message": "Transaction is temporarily banned"},
                            {"code": 1013, "message": "Already Imported"}
                        ])),
                    ),
                ),
            ));

            assert_eq!(
                Client::transaction_pool_submission_outcome(&error),
                None,
                "unexpectedly trusted array JSON data under outer code {outer_code}"
            );
        }
    }

    #[test]
    fn rejects_jsonrpc_parse_and_protocol_error_codes_with_pool_data() {
        for error_object in [
            jsonrpsee::types::error::ErrorObject::owned(
                -32700,
                "Transaction is temporarily banned",
                Some(serde_json::json!({
                    "code": 1012,
                    "message": "Transaction is temporarily banned",
                })),
            ),
            jsonrpsee::types::error::ErrorObject::owned(
                -32600,
                "Already Imported",
                Some(serde_json::json!({
                    "code": 1013,
                    "message": "Already Imported",
                })),
            ),
        ] {
            let error = subxt_client_error(jsonrpsee::core::Error::Call(
                jsonrpsee::types::error::CallError::Custom(error_object),
            ));

            assert_eq!(Client::transaction_pool_submission_outcome(&error), None);
        }
    }

    #[test]
    fn rejects_rpc_subscription_dropped() {
        let error = subxt::Error::Rpc(subxt::error::RpcError::SubscriptionDropped);

        assert!(!Client::is_transaction_imported_or_banned(&error));
    }

    #[test]
    fn rejects_transaction_validity_errors() {
        use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};

        let error =
            subxt::Error::Invalid(TransactionValidityError::Invalid(InvalidTransaction::Stale));

        assert!(!Client::is_transaction_imported_or_banned(&error));
    }

    #[test]
    fn rejects_serialization_and_metadata_independent_subxt_errors() {
        let serialization_error =
            serde_json::from_str::<serde_json::Value>("not-json").expect_err("invalid json");
        let errors = [
            subxt::Error::Serialization(serialization_error),
            subxt::Error::Metadata(subxt::error::MetadataError::IncompatibleMetadata),
            subxt::Error::Other("Already Imported".to_string()),
        ];

        for error in errors {
            assert!(!Client::is_transaction_imported_or_banned(&error));
        }
    }

    #[test]
    fn rejects_non_rpc_subxt_errors_with_pool_like_text() {
        let error = subxt::Error::Other("Transaction is temporarily banned".to_string());

        assert!(!Client::is_transaction_imported_or_banned(&error));
    }

    #[test]
    fn detects_context_wrapped_pool_errors_after_submit_failure() {
        let error = anyhow::Error::new(subxt_pool_error(1012, "Transaction is temporarily banned"))
            .context("sign and submit then watch");

        let subxt_error = error
            .downcast_ref::<subxt::Error>()
            .expect("context should preserve the wrapped subxt error");

        assert!(Client::is_transaction_imported_or_banned(subxt_error));
    }
}
