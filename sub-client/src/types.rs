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

use sp_core::H256;

#[derive(Debug, Clone, Copy)]
pub enum BlockNumberOrHash {
    Number(u64),
    Hash(H256),
    Best,
    Finalized,
}

impl std::fmt::Display for BlockNumberOrHash {
    fn fmt(
        &self,
        f: &mut scale_info::prelude::fmt::Formatter<'_>,
    ) -> scale_info::prelude::fmt::Result {
        match self {
            Self::Best => write!(f, "finalized"),
            Self::Hash(hash) => write!(f, "hash:{:?}", hash),
            Self::Number(n) => write!(f, "number:{}", n),
            Self::Finalized => write!(f, "finalized"),
        }
    }
}

impl From<()> for BlockNumberOrHash {
    fn from(_: ()) -> Self {
        BlockNumberOrHash::Best
    }
}

impl From<u64> for BlockNumberOrHash {
    fn from(number: u64) -> Self {
        BlockNumberOrHash::Number(number)
    }
}

impl From<u32> for BlockNumberOrHash {
    fn from(number: u32) -> Self {
        BlockNumberOrHash::Number(number.into())
    }
}

impl From<H256> for BlockNumberOrHash {
    fn from(hash: H256) -> Self {
        BlockNumberOrHash::Hash(hash)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PalletInfo {
    pub name: &'static str,
}

impl PalletInfo {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn prefix(&self) -> [u8; 16] {
        sp_core::twox_128(self.name.as_bytes())
    }
}
