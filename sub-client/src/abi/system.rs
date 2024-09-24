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

use codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_encode::EncodeAsType;

use crate::{tx::SignedTx, types::PalletInfo, SubResult};

#[derive(Clone, Encode, Decode, PartialEq, Eq, EncodeAsType, DecodeAsType)]
pub struct KillPrefix {
    prefix: Vec<u8>,
    subkeys: u32,
}

impl core::fmt::Debug for KillPrefix {
    fn fmt(
        &self,
        f: &mut scale_info::prelude::fmt::Formatter<'_>,
    ) -> scale_info::prelude::fmt::Result {
        f.debug_struct("KillPrefix")
            .field(
                "prefix",
                &sp_core::hexdisplay::HexDisplay::from(&self.prefix),
            )
            .field("subkeys", &self.subkeys)
            .finish()
    }
}

const PALLET: PalletInfo = PalletInfo::new("System");

const KILL_PREFIX_CALL: SignedTx<KillPrefix> = SignedTx::new(PALLET, "kill_prefix");

#[async_trait::async_trait]
pub trait SystemTx<T: subxt::Config> {
    async fn kill_prefix(&self, prefix: Vec<u8>, subkeys: u32) -> SubResult<()>;
}

#[async_trait::async_trait]
impl<T, P> SystemTx<T> for crate::tx::SignedTxs<T, P>
where
    T: subxt::Config<ExtrinsicParams = subxt::config::DefaultExtrinsicParams<T>>,
    P: sp_core::Pair + Send + Sync + Clone,
    T::Signature: From<P::Signature> + Send + Sync,
    T::AccountId: From<sp_runtime::AccountId32> + Send + Sync,
    T::AssetId: Send + Sync,
{
    async fn kill_prefix(&self, prefix: Vec<u8>, subkeys: u32) -> SubResult<()> {
        KILL_PREFIX_CALL
            .submit_sudo(self, KillPrefix { prefix, subkeys })
            .await
    }
}
