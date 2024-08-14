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

use bridge_types::ton::TonAddress;
use bridge_types::ton::TonNetworkId;
use bridge_types::EVMChainId;
use bridge_types::GenericNetworkId;
use bridge_types::SubNetworkId;
use codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_encode::EncodeAsType;
use scale_info::TypeInfo;
use sp_core::Pair;
use sp_core::H160;
use sp_core::H256;
use sp_runtime::traits::Hash;
use subxt::utils::Static as StaticType;

use crate::abi::multisig::MultisigUnsignedTx;
use crate::config::liberland::LiberlandConfig;
use crate::config::parachain::ParachainConfig;
use crate::config::sora::SoraConfig;
use crate::constant::ConstantEntry;
use crate::error::Error;
use crate::error::SubResult;
use crate::storage::StorageMap;
use crate::tx::SignedTx;
use crate::types::BlockNumberOrHash;
use crate::types::PalletInfo;
use crate::unsigned_tx::UnsignedTx;
use crate::BlockNumberOf;

use super::multisig::MultisigStorage;

pub const SUB_INBOUND_PALLET: PalletInfo = PalletInfo::new("SubstrateBridgeInboundChannel");
pub const SUB_OUTBOUND_PALLET: PalletInfo = PalletInfo::new("SubstrateBridgeOutboundChannel");
pub const INBOUND_PALLET: PalletInfo = PalletInfo::new("BridgeInboundChannel");
pub const OUTBOUND_PALLET: PalletInfo = PalletInfo::new("BridgeOutboundChannel");

pub const SUB_INBOUND_NONCES: StorageMap<SubNetworkId, u64> =
    StorageMap::new(SUB_INBOUND_PALLET, "ChannelNonces");
pub const SUB_OUTBOUND_NONCES: StorageMap<SubNetworkId, u64> =
    StorageMap::new(SUB_OUTBOUND_PALLET, "ChannelNonces");

pub const INBOUND_NONCES: StorageMap<GenericNetworkId, u64> =
    StorageMap::new(INBOUND_PALLET, "ChannelNonces");
pub const OUTBOUND_NONCES: StorageMap<GenericNetworkId, u64> =
    StorageMap::new(OUTBOUND_PALLET, "ChannelNonces");

pub const INBOUND_REPORTED_NONCES: StorageMap<GenericNetworkId, u64> =
    StorageMap::new(INBOUND_PALLET, "ReportedChannelNonces");

pub const fn sub_outbound_commitment<T: subxt::Config>(
) -> StorageMap<SubNetworkId, GenericCommitmentWithBlock<T>, ()>
where
    BlockNumberOf<T>: Decode,
{
    StorageMap::new(SUB_OUTBOUND_PALLET, "LatestCommitment")
}

pub const fn outbound_commitment<T: subxt::Config>(
) -> StorageMap<GenericNetworkId, GenericCommitmentWithBlock<T>, ()>
where
    BlockNumberOf<T>: Decode,
{
    StorageMap::new(OUTBOUND_PALLET, "LatestCommitment")
}

pub const INBOUND_EVM_ADDRESSES: StorageMap<EVMChainId, H160> =
    StorageMap::new(INBOUND_PALLET, "EVMChannelAddresses");

pub const INBOUND_TON_ADDRESSES: StorageMap<TonNetworkId, TonAddress> =
    StorageMap::new(INBOUND_PALLET, "TONChannelAddresses");

pub const SUB_INBOUND_THIS_NETWORK: ConstantEntry<GenericNetworkId> =
    ConstantEntry::new(SUB_INBOUND_PALLET, "ThisNetworkId");

pub const INBOUND_THIS_NETWORK: ConstantEntry<GenericNetworkId> =
    ConstantEntry::new(INBOUND_PALLET, "ThisNetworkId");

pub const INBOUND_SUBMIT_SORA: UnsignedTx<GenericSubmit<StaticType<SoraMultiProof>>> =
    UnsignedTx::new(INBOUND_PALLET, "submit");

pub const SUB_INBOUND_SUBMIT_SORA: UnsignedTx<SubstrateSubmit<StaticType<SoraMultiProof>>> =
    UnsignedTx::new(SUB_INBOUND_PALLET, "submit");
pub const SUB_INBOUND_SUBMIT_LIBERLAND: UnsignedTx<
    SubstrateSubmit<StaticType<LiberlandMultiProof>>,
> = UnsignedTx::new(SUB_INBOUND_PALLET, "submit");
pub const SUB_INBOUND_SUBMIT_PARACHAIN: UnsignedTx<
    SubstrateSubmit<StaticType<ParachainMultiProof>>,
> = UnsignedTx::new(SUB_INBOUND_PALLET, "submit");

pub const INBOUND_REGISTER_EVM: SignedTx<
    GenericRegister<StaticType<EVMChainId>, StaticType<H160>>,
> = SignedTx::new(INBOUND_PALLET, "register_evm_channel");

pub const INBOUND_REGISTER_TON: SignedTx<
    GenericRegister<StaticType<TonNetworkId>, StaticType<TonAddress>>,
> = SignedTx::new(INBOUND_PALLET, "register_ton_channel");

pub type MaxU32 = sp_core::ConstU32<{ std::u32::MAX }>;
pub type GenericCommitment = bridge_types::GenericCommitment<MaxU32, MaxU32>;
pub type GenericCommitmentWithBlock<T> = bridge_types::types::GenericCommitmentWithBlock<
    <<T as subxt::Config>::Header as subxt::config::Header>::Number,
    MaxU32,
    MaxU32,
>;

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct BeefyProof {
    pub proof: bridge_common::simplified_proof::Proof<H256>,
    pub leaf: bridge_common::beefy_types::BeefyMMRLeaf,
    pub digest: bridge_types::types::AuxiliaryDigest,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct SubProof {
    pub digest: bridge_types::types::AuxiliaryDigest,
    pub proof: Vec<sp_core::ecdsa::Signature>,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct EVMProof {
    pub proof: Vec<sp_core::ecdsa::Signature>,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum MultiProof {
    Beefy(BeefyProof),
    Sub(SubProof),
    EVM(EVMProof),
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum LiberlandMultiProof {
    Multisig(SubProof),
}

pub type ParachainMultiProof = SubProof;

pub type SoraMultiProof = MultiProof;

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct GenericSubmit<Proof> {
    pub network_id: StaticType<GenericNetworkId>,
    pub commitment: StaticType<GenericCommitment>,
    pub proof: Proof,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct SubstrateSubmit<Proof> {
    pub network_id: StaticType<SubNetworkId>,
    pub commitment: StaticType<GenericCommitment>,
    pub proof: Proof,
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct GenericRegister<Network, Channel> {
    pub network_id: Network,
    pub channel_address: Channel,
}

#[async_trait::async_trait]
pub trait ChannelUnsignedTx<T: subxt::Config> {
    async fn submit(
        &self,
        network_id: GenericNetworkId,
        commitment: GenericCommitment,
        proof: MultiProof,
    ) -> SubResult<()>;
}

#[async_trait::async_trait]
pub trait ChannelSignedTx<T: subxt::Config> {
    async fn register_evm_channel(
        &self,
        network_id: GenericNetworkId,
        channel: H160,
    ) -> SubResult<()>;

    async fn register_ton_channel(
        &self,
        network_id: GenericNetworkId,
        channel: TonAddress,
    ) -> SubResult<()>;
}

#[async_trait::async_trait]
pub trait ChannelStorage<T: subxt::Config> {
    async fn latest_commitment(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<GenericCommitmentWithBlock<T>>>;

    async fn inbound_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64>;

    async fn outbound_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64>;

    async fn reported_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64>;

    async fn evm_channel_address(&self, network_id: GenericNetworkId) -> SubResult<Option<H160>>;

    async fn ton_channel_address(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<TonAddress>>;
}

#[async_trait::async_trait]
pub trait ChannelConstants<T: subxt::Config> {
    async fn network_id(&self) -> SubResult<GenericNetworkId>;
}

#[async_trait::async_trait]
impl<T: subxt::Config> ChannelStorage<T> for crate::Storages<T>
where
    <<T as subxt::Config>::Header as subxt::config::Header>::Number: Decode + Send + Sync,
{
    async fn latest_commitment(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<GenericCommitmentWithBlock<T>>> {
        match network_id {
            GenericNetworkId::Sub(network_id)
                if sub_outbound_commitment::<T>().is_supported(self) =>
            {
                sub_outbound_commitment::<T>().fetch(self, network_id).await
            }
            network_id if outbound_commitment::<T>().is_supported(self) => {
                outbound_commitment::<T>().fetch(self, network_id).await
            }
            _ => Err(Error::NotSupported(format!(
                "{:?} or {:?}",
                sub_outbound_commitment::<T>(),
                outbound_commitment::<T>()
            ))),
        }
    }

    async fn outbound_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64> {
        match network_id {
            GenericNetworkId::Sub(network_id) if SUB_OUTBOUND_NONCES.is_supported(self) => {
                SUB_OUTBOUND_NONCES.fetch_or_default(self, network_id).await
            }
            network_id if OUTBOUND_NONCES.is_supported(self) => {
                OUTBOUND_NONCES.fetch_or_default(self, network_id).await
            }
            _ => Err(Error::NotSupported(format!(
                "{:?} or {:?}",
                SUB_OUTBOUND_NONCES, OUTBOUND_NONCES
            ))),
        }
    }

    async fn inbound_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64> {
        match network_id {
            GenericNetworkId::Sub(network_id) if SUB_INBOUND_NONCES.is_supported(self) => {
                SUB_INBOUND_NONCES.fetch_or_default(self, network_id).await
            }
            network_id if INBOUND_NONCES.is_supported(self) => {
                INBOUND_NONCES.fetch_or_default(self, network_id).await
            }
            _ => Err(Error::NotSupported(format!(
                "{:?} or {:?}",
                SUB_INBOUND_NONCES, INBOUND_NONCES
            ))),
        }
    }

    async fn reported_nonce(&self, network_id: GenericNetworkId) -> SubResult<u64> {
        INBOUND_REPORTED_NONCES
            .fetch_or_default(self, network_id)
            .await
    }

    async fn evm_channel_address(&self, network_id: GenericNetworkId) -> SubResult<Option<H160>> {
        match network_id {
            GenericNetworkId::EVM(chain_id) => INBOUND_EVM_ADDRESSES.fetch(self, chain_id).await,
            network_id => Err(Error::NetworkNotSupported(network_id)),
        }
    }

    async fn ton_channel_address(
        &self,
        network_id: GenericNetworkId,
    ) -> SubResult<Option<TonAddress>> {
        match network_id {
            GenericNetworkId::TON(network_id) => {
                INBOUND_TON_ADDRESSES.fetch(self, network_id).await
            }
            network_id => Err(Error::NetworkNotSupported(network_id)),
        }
    }
}

#[async_trait::async_trait]
impl<P> ChannelSignedTx<SoraConfig> for crate::tx::SignedTxs<SoraConfig, P>
where
    P: sp_core::Pair + Send + Sync + Clone,
    sp_runtime::MultiSignature: From<<P as sp_core::Pair>::Signature>,
{
    async fn register_evm_channel(
        &self,
        network_id: GenericNetworkId,
        channel: H160,
    ) -> SubResult<()> {
        match network_id {
            GenericNetworkId::EVM(network_id) => {
                INBOUND_REGISTER_EVM
                    .submit_sudo(
                        self,
                        GenericRegister {
                            network_id: StaticType(network_id),
                            channel_address: StaticType(channel),
                        },
                    )
                    .await
            }
            network_id => Err(Error::NetworkNotSupported(network_id)),
        }
    }

    async fn register_ton_channel(
        &self,
        network_id: GenericNetworkId,
        channel: TonAddress,
    ) -> SubResult<()> {
        match network_id {
            GenericNetworkId::TON(network_id) => {
                INBOUND_REGISTER_TON
                    .submit_sudo(
                        self,
                        GenericRegister {
                            network_id: StaticType(network_id),
                            channel_address: StaticType(channel),
                        },
                    )
                    .await
            }
            network_id => Err(Error::NetworkNotSupported(network_id)),
        }
    }
}

#[async_trait::async_trait]
impl ChannelUnsignedTx<SoraConfig> for crate::unsigned_tx::UnsignedTxs<SoraConfig> {
    async fn submit(
        &self,
        network_id: GenericNetworkId,
        commitment: GenericCommitment,
        proof: MultiProof,
    ) -> SubResult<()> {
        match network_id {
            GenericNetworkId::Sub(network_id) if SUB_INBOUND_SUBMIT_SORA.is_supported(self) => {
                SUB_INBOUND_SUBMIT_SORA
                    .submit(
                        self,
                        SubstrateSubmit {
                            network_id: StaticType(network_id),
                            commitment: StaticType(commitment),
                            proof: StaticType(proof),
                        },
                    )
                    .await
            }
            network_id if INBOUND_SUBMIT_SORA.is_supported(self) => {
                INBOUND_SUBMIT_SORA
                    .submit(
                        self,
                        GenericSubmit {
                            network_id: StaticType(network_id),
                            commitment: StaticType(commitment),
                            proof: StaticType(proof),
                        },
                    )
                    .await
            }
            _ => Err(Error::NotSupported(format!(
                "{:?} or {:?}",
                SUB_INBOUND_SUBMIT_SORA, INBOUND_SUBMIT_SORA
            ))),
        }
    }
}

#[async_trait::async_trait]
impl ChannelUnsignedTx<ParachainConfig> for crate::unsigned_tx::UnsignedTxs<ParachainConfig> {
    async fn submit(
        &self,
        network_id: GenericNetworkId,
        commitment: GenericCommitment,
        proof: MultiProof,
    ) -> SubResult<()> {
        match (network_id, proof) {
            (GenericNetworkId::Sub(network_id), MultiProof::Sub(proof)) => {
                SUB_INBOUND_SUBMIT_PARACHAIN
                    .submit(
                        self,
                        SubstrateSubmit {
                            network_id: StaticType(network_id),
                            commitment: StaticType(commitment),
                            proof: StaticType(proof),
                        },
                    )
                    .await
            }
            (network_id, MultiProof::Sub(_)) => Err(Error::NetworkNotSupported(network_id)),
            _ => Err(Error::ProofNotSupported),
        }
    }
}

#[async_trait::async_trait]
impl ChannelUnsignedTx<LiberlandConfig> for crate::unsigned_tx::UnsignedTxs<LiberlandConfig> {
    async fn submit(
        &self,
        network_id: GenericNetworkId,
        commitment: GenericCommitment,
        proof: MultiProof,
    ) -> SubResult<()> {
        match (network_id, proof) {
            (GenericNetworkId::Sub(network_id), MultiProof::Sub(proof)) => {
                SUB_INBOUND_SUBMIT_LIBERLAND
                    .submit(
                        self,
                        SubstrateSubmit {
                            network_id: StaticType(network_id),
                            commitment: StaticType(commitment),
                            proof: StaticType(LiberlandMultiProof::Multisig(proof)),
                        },
                    )
                    .await
            }
            (network_id, MultiProof::Sub(_)) => Err(Error::NetworkNotSupported(network_id)),
            _ => Err(Error::ProofNotSupported),
        }
    }
}

#[async_trait::async_trait]
impl<T: subxt::Config> ChannelConstants<T> for crate::Constants<T> {
    async fn network_id(&self) -> SubResult<GenericNetworkId> {
        if SUB_INBOUND_THIS_NETWORK.is_supported(self) {
            SUB_INBOUND_THIS_NETWORK.fetch(self)
        } else if INBOUND_THIS_NETWORK.is_supported(self) {
            INBOUND_THIS_NETWORK.fetch(self)
        } else {
            Err(Error::NotSupported(format!(
                "{:?} or {:?}",
                SUB_INBOUND_THIS_NETWORK, INBOUND_THIS_NETWORK
            )))
        }
    }
}

impl<T> crate::UnsignedClient<T>
where
    T: subxt::Config,
    crate::Storages<T>: ChannelStorage<T>,
{
    #[instrument(skip(self), err(level = "warn"))]
    pub async fn commitment_with_nonce(
        &self,
        network_id: GenericNetworkId,
        nonce: u64,
    ) -> SubResult<GenericCommitmentWithBlock<T>> {
        let mut client = self.at(BlockNumberOrHash::Finalized).await?;
        loop {
            let commitment = client
                .storage()
                .await?
                .latest_commitment(network_id)
                .await?
                .ok_or(Error::CommitmentNotFound(nonce))?;
            if commitment.commitment.nonce() == nonce {
                return Ok(commitment);
            } else if commitment.commitment.nonce() < nonce {
                return Err(Error::CommitmentNotFound(nonce));
            } else {
                client = client
                    .at(BlockNumberOrHash::Number(commitment.block_number.into()))
                    .await?;
            }
        }
    }
}

impl<T> crate::UnsignedClient<T>
where
    T: subxt::Config,
    crate::Storages<T>: ChannelStorage<T> + MultisigStorage<T>,
    crate::UnsignedTxs<T>: ChannelUnsignedTx<T> + MultisigUnsignedTx<T>,
{
    pub async fn submit_inbound_commitment(
        &self,
        signer: sp_core::ecdsa::Pair,
        sender: GenericNetworkId,
        receiver: GenericNetworkId,
        commitment: GenericCommitment,
    ) -> SubResult<()> {
        info!("Submit commitment {commitment:?}");
        let message =
            sp_runtime::traits::Keccak256::hash_of(&(sender, receiver, commitment.hash()));
        self.approve_message(signer, sender, message).await?;
        if self.should_send_commitment(sender, message).await? {
            info!("Sending commitment");
            let approvals = self
                .storage()
                .await?
                .approvals(sender, message)
                .await?
                .into_iter()
                .map(|(_, s)| s)
                .collect::<Vec<_>>();
            let proof = MultiProof::EVM(EVMProof { proof: approvals });
            self.unsigned_tx()
                .await?
                .submit(sender, commitment, proof)
                .await?;
        }
        Ok(())
    }

    #[instrument(skip(self, signer), err(level = "warn"), fields(signer = signer.public().to_string()))]
    pub async fn approve_message(
        &self,
        signer: sp_core::ecdsa::Pair,
        sender: GenericNetworkId,
        message: H256,
    ) -> SubResult<()> {
        if self
            .should_send_approval(sender, signer.public(), message)
            .await?
        {
            info!("Sending approval");
            let signature = signer.sign_prehashed(&message.0);
            self.unsigned_tx()
                .await?
                .approve(sender, message, signature)
                .await?;
        } else {
            info!("Already approved");
        }
        Ok(())
    }

    pub async fn should_send_approval(
        &self,
        network_id: GenericNetworkId,
        signer: sp_core::ecdsa::Public,
        message: H256,
    ) -> SubResult<bool> {
        let peers = self
            .storage()
            .await?
            .peers(network_id)
            .await?
            .ok_or(Error::NetworkNotRegistered(network_id))?;
        let approvals = self.storage().await?.approvals(network_id, message).await?;
        let is_already_approved = approvals
            .iter()
            .any(|(public, _signature)| signer == *public);
        Ok(
            (approvals.len() as u32) < bridge_types::utils::threshold(peers.len() as u32)
                && !is_already_approved,
        )
    }

    pub async fn should_send_commitment(
        &self,
        network_id: GenericNetworkId,
        message: H256,
    ) -> SubResult<bool> {
        let peers = self
            .storage()
            .await?
            .peers(network_id)
            .await?
            .ok_or(Error::NetworkNotRegistered(network_id))?;
        let approvals = self.storage().await?.approvals(network_id, message).await?;
        Ok((approvals.len() as u32) >= bridge_types::utils::threshold(peers.len() as u32))
    }
}
