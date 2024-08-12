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

use thiserror::Error;
use toner::tlb::{ser::CellBuilderError, StringError};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid http header value: {0}")]
    InvalidHttpHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),
    #[error("Url parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("[{0}] Missing result field in response")]
    MissingResult(String),
    #[error("[{0}] Response error with code {1}: {2}")]
    RequestFailed(String, i64, String),
    #[error("Wrong key format")]
    WrongKeyFormat,
    #[error("Wrong wallet version: {0}")]
    WrongWalletVersion(String),
    #[error("Serde json: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Wrong stack")]
    WrongStack,
    #[error("Contract get error: {0}")]
    ContractGet(i64),
    #[error("Cell builder error: {0}")]
    CellBuilderError(CellBuilderError),
    #[error("Toner string error: {0}")]
    TonerStringError(#[from] StringError),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

pub type TonResult<T> = std::result::Result<T, Error>;
