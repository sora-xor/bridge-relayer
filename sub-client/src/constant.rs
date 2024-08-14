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

use codec::Decode;
use subxt::{constants::StaticAddress, utils::Static};

use crate::{types::PalletInfo, Error, SubResult, UnsignedClient};

pub struct Constants<T: subxt::Config> {
    client: UnsignedClient<T>,
    constants: subxt::constants::ConstantsClient<T, subxt::OnlineClient<T>>,
}

impl<T: subxt::Config> Constants<T> {
    pub fn from_client(client: UnsignedClient<T>) -> Self {
        Self {
            constants: client.inner.constants(),
            client,
        }
    }

    pub fn client(&self) -> UnsignedClient<T> {
        self.client.clone()
    }

    pub fn constants(&self) -> subxt::constants::ConstantsClient<T, subxt::OnlineClient<T>> {
        self.constants.clone()
    }

    pub fn is_supported(&self, pallet: &str, entry: &str) -> bool {
        let metadata = self.client().metadata();
        metadata
            .pallet_by_name(pallet)
            .and_then(|p| p.constant_by_name(entry))
            .is_some()
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct ConstantEntry<R> {
    pub pallet: PalletInfo,
    pub entry: &'static str,
    _phantom: PhantomData<R>,
}

impl<R> ConstantEntry<R>
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

    pub fn address(&self) -> StaticAddress<Static<R>> {
        StaticAddress::new_static(self.pallet.name, self.entry, [0u8; 32]).unvalidated()
    }

    #[instrument(skip(consts))]
    pub fn fetch<T: subxt::Config>(&self, consts: &Constants<T>) -> SubResult<R> {
        if self.is_supported(consts) {
            Ok(consts.constants().at(&self.address())?.0)
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }

    pub fn is_supported<T: subxt::Config>(&self, consts: &Constants<T>) -> bool {
        consts.is_supported(self.pallet.name, self.entry)
    }
}
