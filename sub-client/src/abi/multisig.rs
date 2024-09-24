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

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bridge_types::GenericNetworkId;
use codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_encode::EncodeAsType;
use sp_core::H256;
use subxt::utils::Static as StaticType;

use crate::error::SubResult;
use crate::storage::StorageDoubleMap;
use crate::storage::StorageMap;
use crate::tx::SignedTx;
use crate::types::PalletInfo;
use crate::unsigned_tx::UnsignedTx;

pub const SIGNER_PALLET: PalletInfo = PalletInfo::new("BridgeDataSigner");
pub const VERIFIER_PALLET: PalletInfo = PalletInfo::new("MultisigVerifier");

pub const SIGNER_APPROVALS: StorageDoubleMap<
    GenericNetworkId,
    H256,
    BTreeMap<sp_core::ecdsa::Public, sp_core::ecdsa::Signature>,
> = StorageDoubleMap::new(SIGNER_PALLET, "Approvals");
pub const SIGNER_PEERS: StorageMap<GenericNetworkId, BTreeSet<sp_core::ecdsa::Public>, ()> =
    StorageMap::new(SIGNER_PALLET, "Peers");

pub const APPROVE_CALL: UnsignedTx<Approve> = UnsignedTx::new(SIGNER_PALLET, "approve");

pub const INITIALIZE_CALL: SignedTx<Register> = SignedTx::new(VERIFIER_PALLET, "initialize");
pub const REGISTER_CALL: SignedTx<Register> = SignedTx::new(SIGNER_PALLET, "register_network");

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct Register {
    pub network_id: StaticType<GenericNetworkId>,
    pub peers: StaticType<Vec<sp_core::ecdsa::Public>>,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct Approve {
    pub network_id: StaticType<GenericNetworkId>,
    pub data: StaticType<H256>,
    pub signature: StaticType<sp_core::ecdsa::Signature>,
}

#[async_trait::async_trait]
pub trait MultisigUnsignedTx<T: subxt::Config> {
    async fn approve(
        &self,
        network_id: GenericNetworkId,
        message: H256,
        signature: sp_core::ecdsa::Signature,
    ) -> SubResult<()>;
}

#[async_trait::async_trait]
pub trait MultisigTx<T: subxt::Config> {
    async fn register_signer(
        &self,
        network_id: GenericNetworkId,
        peers: Vec<sp_core::ecdsa::Public>,
    ) -> SubResult<()>;

    async fn register_verifier(
        &self,
        network_id: GenericNetworkId,
        peers: Vec<sp_core::ecdsa::Public>,
    ) -> SubResult<()>;
}

#[async_trait::async_trait]
pub trait MultisigStorage<T: subxt::Config> {
    async fn approvals(
        &self,
        network_id: GenericNetworkId,
        message: H256,
    ) -> SubResult<BTreeMap<sp_core::ecdsa::Public, sp_core::ecdsa::Signature>>;

    async fn peers(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<BTreeSet<sp_core::ecdsa::Public>>>;
}

#[async_trait::async_trait]
impl<T: subxt::Config> MultisigStorage<T> for crate::Storages<T> {
    async fn approvals(
        &self,
        network_id: GenericNetworkId,
        message: H256,
    ) -> SubResult<BTreeMap<sp_core::ecdsa::Public, sp_core::ecdsa::Signature>> {
        SIGNER_APPROVALS
            .fetch_or_default(self, network_id, message)
            .await
    }

    async fn peers(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<BTreeSet<sp_core::ecdsa::Public>>> {
        SIGNER_PEERS.fetch(self, network_id).await
    }
}

#[async_trait::async_trait]
impl<T: subxt::Config> MultisigUnsignedTx<T> for crate::unsigned_tx::UnsignedTxs<T> {
    async fn approve(
        &self,
        network_id: GenericNetworkId,
        message: H256,
        signature: sp_core::ecdsa::Signature,
    ) -> SubResult<()> {
        APPROVE_CALL
            .submit(
                self,
                Approve {
                    network_id: StaticType(network_id),
                    data: StaticType(message),
                    signature: StaticType(signature),
                },
            )
            .await
    }
}

#[async_trait::async_trait]
impl<T, P> MultisigTx<T> for crate::tx::SignedTxs<T, P>
where
    T: subxt::Config<ExtrinsicParams = subxt::config::DefaultExtrinsicParams<T>>,
    P: sp_core::Pair + Send + Sync + Clone,
    T::Signature: From<P::Signature> + Send + Sync,
    T::AccountId: From<sp_runtime::AccountId32> + Send + Sync,
    T::AssetId: Send + Sync,
{
    async fn register_signer(
        &self,
        network_id: GenericNetworkId,
        peers: Vec<sp_core::ecdsa::Public>,
    ) -> SubResult<()> {
        REGISTER_CALL
            .submit_sudo(
                self,
                Register {
                    network_id: StaticType(network_id),
                    peers: StaticType(peers),
                },
            )
            .await
    }

    async fn register_verifier(
        &self,
        network_id: GenericNetworkId,
        peers: Vec<sp_core::ecdsa::Public>,
    ) -> SubResult<()> {
        INITIALIZE_CALL
            .submit_sudo(
                self,
                Register {
                    network_id: StaticType(network_id),
                    peers: StaticType(peers),
                },
            )
            .await
    }
}
