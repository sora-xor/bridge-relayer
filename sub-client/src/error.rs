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

use bridge_types::GenericNetworkId;
use thiserror::Error;

use crate::types::BlockNumberOrHash;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Subxt error: {0}")]
    Subxt(#[from] subxt::Error),
    #[error("Subxt core error: {0}")]
    SubxtCore(#[from] subxt::ext::subxt_core::Error),
    #[error("Subxt metadata error: {0}")]
    SubxtMetadata(#[from] subxt::ext::subxt_core::error::MetadataError),
    #[error("Codec error: {0}")]
    Codec(#[from] codec::Error),
    #[error("Tx error: {0}")]
    TxSubmit(String),
    #[error("Tx invalid: {0}")]
    TxInvalid(String),
    #[error("Tx dropped: {0}")]
    TxDropped(String),
    #[error("Missing tx status")]
    TxStatusMissing,
    #[error("Block {0} not found")]
    BlockNotFound(BlockNumberOrHash),
    #[error("Call not supported: {0}")]
    NotSupported(String),
    #[error("Network not supported: {0:?}")]
    NetworkNotSupported(GenericNetworkId),
    #[error("Network not registered: {0:?}")]
    NetworkNotRegistered(GenericNetworkId),
    #[error("Proof type not supported")]
    ProofNotSupported,
    #[error("Commitment with nonce {0} not found")]
    CommitmentNotFound(u64),
    #[error("Digest is empty")]
    EmptyDigest,
    #[error("Digest not found")]
    DigestNotFound,
}

pub type SubResult<T> = std::result::Result<T, Error>;
