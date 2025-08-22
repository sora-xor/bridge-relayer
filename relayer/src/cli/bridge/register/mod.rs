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
//! Register bridge apps and channels across networks.
//!
//! Typical flow:
//! - Register apps on SORA for the target network (e.g., ERC20/Eth app).
//! - Initialize or reset the channel contract on EVM/TON.
//! - Register channel addresses on SORA for the external network.
//!
//! Examples:
//! - SORA ↔ EVM fungible app (new asset):
//!   `bridge register sora evm app fungible-new --contract 0x... --name DAI --symbol DAI --precision 18`
//! - Initialize EVM channel:
//!   `bridge register evm initialize-channels --evm-url http://...`
//! - SORA ↔ TON app and channels:
//!   `bridge register sora ton app ...` then `bridge register sora ton channels ...`
mod evm;
mod liberland;
mod parachain;
mod sora;
mod ton;

use crate::cli::prelude::*;
use clap::*;

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Register operations in EVM network
    #[clap(subcommand)]
    EVM(evm::Commands),
    /// Register operations in SORA network
    #[clap(subcommand)]
    Sora(sora::Commands),
    /// Register operations in parachain
    #[clap(subcommand)]
    Parachain(parachain::Commands),
    /// Register operations in liberland
    #[clap(subcommand)]
    Liberland(liberland::Commands),
    /// Register operations in TON network
    #[clap(subcommand)]
    TON(ton::Commands),
}

impl Commands {
    pub async fn run(&self) -> AnyResult<()> {
        match self {
            Commands::EVM(cmd) => cmd.run().await,
            Commands::Sora(cmd) => cmd.run().await,
            Commands::Parachain(cmd) => cmd.run().await,
            Commands::Liberland(cmd) => cmd.run().await,
            Commands::TON(cmd) => cmd.run().await,
        }
    }
}
