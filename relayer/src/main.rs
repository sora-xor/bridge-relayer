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

mod cli;
mod metrics;
mod relay;
use clap::Parser;
use prelude::*;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate anyhow;

#[tokio::main]
async fn main() -> AnyResult<()> {
    init_log();
    let cli = cli::Cli::parse();
    cli.run().await.map_err(|e| {
        error!("Relayer returned error: {:?}", e);
        e
    })?;
    Ok(())
}

fn init_log() {
    use tracing::Level;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

pub mod prelude {
    pub use crate::metrics::*;
    pub use anyhow::{Context, Result as AnyResult};
    pub use codec::{Decode, Encode};
    pub use either::Either;
    pub use evm_client::alloy;
    pub use evm_client::alloy::primitives as evm_primitives;
    pub use evm_client::Client as EvmClient;
    pub use evm_primitives::Address as EvmAddress;
    pub use hex_literal::hex;
    pub use http::Uri;
    pub use serde::{Deserialize, Serialize};
    pub use sub_client::abi::channel::GenericCommitment;
    pub use sub_client::bridge_types;
    pub use sub_client::{
        abi::{
            channel::{ChannelConstants, ChannelStorage, ChannelUnsignedTx},
            multisig::{MultisigStorage, MultisigTx, MultisigUnsignedTx},
        },
        config::{liberland::LiberlandConfig, parachain::ParachainConfig, sora::SoraConfig},
        error::Error as SubError,
        sp_core,
        sp_core::{ecdsa, sr25519, Pair as CryptoPair},
        sp_runtime, Constants as SubConstants, SignedClient as SubSignedClient,
        SignedTxs as SubSignedTxs, Storages as SubStorage, UnsignedClient as SubUnsignedClient,
        UnsignedTxs as SubUnsignedTxs,
    };
    pub use url::Url;
}
