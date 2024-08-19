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
    types::{AuxiliaryDigest, AuxiliaryDigestItem},
    GenericNetworkId,
};
use sp_core::H256;

use crate::{
    storage::StorageEntry,
    types::{BlockNumberOrHash, PalletInfo},
    Error, SubResult,
};

const PALLET: PalletInfo = PalletInfo::new("LeafProvider");

const LATEST_DIGEST: StorageEntry<AuxiliaryDigest, ()> = StorageEntry::new(PALLET, "LatestDigest");

#[async_trait::async_trait]
pub trait ChannelStorage<T: subxt::Config> {
    async fn latest_digest(&self) -> SubResult<Option<AuxiliaryDigest>>;
}

#[async_trait::async_trait]
impl<T: subxt::Config> ChannelStorage<T> for crate::Storages<T> {
    async fn latest_digest(&self) -> SubResult<Option<AuxiliaryDigest>> {
        LATEST_DIGEST.fetch(self).await
    }
}

impl<T: subxt::Config> crate::UnsignedClient<T> {
    pub async fn load_digest(
        &self,
        network_id: GenericNetworkId,
        block_number: u64,
        commitment_hash: H256,
    ) -> SubResult<AuxiliaryDigest> {
        let client = self.at(BlockNumberOrHash::Number(block_number)).await?;
        let digest = client.storage().await?.latest_digest().await?;
        let Some(digest) = digest else {
            return Err(Error::EmptyDigest);
        };
        let valid_items = digest
            .logs
            .iter()
            .filter(|log| {
                let AuxiliaryDigestItem::Commitment(digest_network_id, digest_commitment_hash) =
                    log;
                !(network_id != *digest_network_id && commitment_hash != *digest_commitment_hash)
            })
            .count();
        if valid_items != 1 {
            return Err(Error::DigestNotFound);
        }
        Ok(digest)
    }
}
