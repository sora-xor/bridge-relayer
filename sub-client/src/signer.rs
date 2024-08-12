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

use sp_core::Pair as PairT;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    AccountId32, MultiSignature,
};
use subxt::tx::Signer;
use subxt::Config;

/// A [`Signed`] implementation that can be constructed from an [`sp_core::Pair`].
#[derive(Debug, Clone)]
pub struct Signed<Pair: Send + Sync + 'static> {
    account_id: AccountId32,
    signer: Pair,
}

impl<Pair> Signed<Pair>
where
    Pair: PairT + Send + Sync + 'static,
    <MultiSignature as Verify>::Signer: From<Pair::Public>,
{
    /// Creates a new [`Signed`] from an [`sp_core::Pair`].
    pub fn new(signer: Pair) -> Self {
        let account_id = <MultiSignature as Verify>::Signer::from(signer.public()).into_account();
        Self {
            account_id: account_id,
            signer,
        }
    }
}

impl<T, Pair> Signer<T> for Signed<Pair>
where
    T: Config,
    Pair: PairT + Send + Sync + 'static,
    Pair::Signature: Into<T::Signature>,
    T::AccountId: From<AccountId32>,
{
    fn account_id(&self) -> T::AccountId {
        self.account_id.clone().into()
    }

    fn address(&self) -> T::Address {
        Signer::<T>::account_id(self).into()
    }

    fn sign(&self, signer_payload: &[u8]) -> T::Signature {
        self.signer.sign(signer_payload).into()
    }
}

#[derive(Clone, Debug)]
pub struct Unsigned;

impl<T> Signer<T> for Unsigned
where
    T: Config,
{
    fn account_id(&self) -> T::AccountId {
        unimplemented!()
    }

    fn address(&self) -> T::Address {
        unimplemented!()
    }

    fn sign(&self, _: &[u8]) -> T::Signature {
        unimplemented!()
    }
}
