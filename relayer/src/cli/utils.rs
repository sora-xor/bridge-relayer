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

//! Shared CLI helpers and typed client builders.
//!
//! This module provides:
//! - `Network`: flag parsing for selecting a known or custom EVM network config
//! - `SubstrateClient`/`ParachainClient`/`LiberlandClient`: WS RPC + signer wiring
//! - `EthereumClient`: WS/HTTP provider + signer wiring
//!
//! Each client offers `get_unsigned_*` and `get_signed_*` convenience methods that reuse
//! global CLI options and perform basic validation.

use std::path::PathBuf;

use super::error::*;
use crate::{prelude::*, substrate::traits::KeyPair};
use bridge_types::network_config::NetworkConfig;
use clap::error::ErrorKind;
use clap::*;

/// EVM network selection for Ethash light client registration.
#[derive(Clone, Debug)]
pub enum Network {
    Mainnet,
    Ropsten,
    Sepolia,
    Rinkeby,
    Goerli,
    Classic,
    Mordor,
    Custom { path: PathBuf },
    None,
}

const NETWORKS: [&str; 8] = [
    "mainnet", "ropsten", "sepolia", "rinkeby", "goerli", "classic", "mordor", "custom",
];

impl Args for Network {
    fn augment_args(app: Command) -> Command {
        let mut app = app;
        for network in NETWORKS.iter() {
            let mut arg = Arg::new(*network).long(network).required(false);
            if *network == "custom" {
                arg = arg.value_name("PATH").action(ArgAction::SetTrue);
            }
            app = app.arg(arg);
        }
        app
    }

    fn augment_args_for_update(app: Command) -> Command {
        Self::augment_args(app)
    }
}

impl FromArgMatches for Network {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        let mut network = None;
        let mut occurrences = 0;
        for network_name in NETWORKS.iter() {
            if matches.contains_id(network_name) {
                occurrences += 1;
                if occurrences > 1 {
                    return Err(Error::raw(
                        ErrorKind::ArgumentConflict,
                        "Only one network can be specified at a time",
                    ));
                }
                network = Some(match *network_name {
                    "mainnet" => Network::Mainnet,
                    "ropsten" => Network::Ropsten,
                    "sepolia" => Network::Sepolia,
                    "rinkeby" => Network::Rinkeby,
                    "goerli" => Network::Goerli,
                    "classic" => Network::Classic,
                    "mordor" => Network::Mordor,
                    "custom" => {
                        let path: &String = matches.get_one(network_name).expect("required value");
                        Network::Custom {
                            path: PathBuf::from(path),
                        }
                    }
                    _ => unreachable!(),
                });
            }
        }
        Ok(network.unwrap_or(Network::None))
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Network {
    /// Load a `bridge_types::network_config::NetworkConfig` for the selected network.
    ///
    /// - Well-known variants return baked-in configs.
    /// - `Custom { path }` reads JSON from disk.
    /// - `None` yields a CLI error.
    pub fn config(&self) -> AnyResult<NetworkConfig> {
        let res = match self {
            Network::Mainnet => NetworkConfig::Mainnet,
            Network::Ropsten => NetworkConfig::Ropsten,
            Network::Sepolia => NetworkConfig::Sepolia,
            Network::Rinkeby => NetworkConfig::Rinkeby,
            Network::Goerli => NetworkConfig::Goerli,
            Network::Classic => NetworkConfig::Classic,
            Network::Mordor => NetworkConfig::Mordor,
            Network::Custom { path } => {
                let bytes = std::fs::read(path)?;
                serde_json::de::from_slice(&bytes)?
            }
            Network::None => {
                return Err(
                    Error::raw(ErrorKind::MissingRequiredArgument, "No network specified").into(),
                )
            }
        };
        Ok(res)
    }
}

/// Reusable parameters and helpers for connecting to a SORA/Substrate node.
#[derive(Args, Debug, Clone)]
pub struct SubstrateClient {
    #[clap(from_global)]
    substrate_key: Option<String>,
    #[clap(from_global)]
    substrate_key_file: Option<String>,
    #[clap(from_global)]
    substrate_url: Option<String>,
}

impl SubstrateClient {
    /// Obtain the ECDSA key URI from flags or a file. Errors if both or none provided.
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.substrate_key, &self.substrate_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::SubstrateKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    /// Return the WebSocket URL to connect to.
    pub fn get_url(&self) -> AnyResult<String> {
        Ok(self
            .substrate_url
            .clone()
            .ok_or(CliError::SubstrateEndpoint)?)
    }

    /// Create an unsigned Subxt client.
    pub async fn get_unsigned_substrate(&self) -> AnyResult<SubUnsignedClient<MainnetConfig>> {
        let sub = SubUnsignedClient::new(self.get_url()?).await?;
        Ok(sub)
    }

    /// Create a signed Subxt client for submitting extrinsics.
    pub async fn get_signed_substrate(&self) -> AnyResult<SubSignedClient<MainnetConfig>> {
        let sub = self
            .get_unsigned_substrate()
            .await?
            .signed(subxt::tx::PairSigner::new(
                KeyPair::from_string(&self.get_key_string()?, None)
                    .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
            ))
            .await?;
        Ok(sub)
    }
}

/// Reusable parameters and helpers for connecting to a Parachain node.
#[derive(Args, Debug, Clone)]
pub struct ParachainClient {
    #[clap(from_global)]
    parachain_key: Option<String>,
    #[clap(from_global)]
    parachain_key_file: Option<String>,
    #[clap(from_global)]
    parachain_url: Option<String>,
}

impl ParachainClient {
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.parachain_key, &self.parachain_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::ParachainKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    pub fn get_url(&self) -> AnyResult<String> {
        Ok(self
            .parachain_url
            .clone()
            .ok_or(CliError::ParachainEndpoint)?)
    }

    pub async fn get_unsigned_substrate(&self) -> AnyResult<SubUnsignedClient<ParachainConfig>> {
        let sub = SubUnsignedClient::new(self.get_url()?).await?;
        Ok(sub)
    }

    pub async fn get_signed_substrate(&self) -> AnyResult<SubSignedClient<ParachainConfig>> {
        let sub = self
            .get_unsigned_substrate()
            .await?
            .signed(subxt::tx::PairSigner::new(
                KeyPair::from_string(&self.get_key_string()?, None)
                    .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
            ))
            .await?;
        Ok(sub)
    }
}

/// Reusable parameters and helpers for connecting to an EVM node.
#[derive(Args, Debug, Clone)]
pub struct EthereumClient {
    #[clap(from_global)]
    ethereum_key: Option<String>,
    #[clap(from_global)]
    ethereum_key_file: Option<String>,
    #[clap(from_global)]
    ethereum_url: Option<Url>,
    #[clap(from_global)]
    gas_metrics_path: Option<PathBuf>,
}

impl EthereumClient {
    /// Get signer private key as a hex string from flags or a file.
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.ethereum_key, &self.ethereum_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::EthereumKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    /// Return the WS/HTTP URL to connect to.
    pub fn get_url(&self) -> AnyResult<Url> {
        Ok(self
            .ethereum_url
            .clone()
            .ok_or(CliError::EthereumEndpoint)?)
    }

    /// Create an unsigned ethers Provider (reads only).
    pub async fn get_unsigned_ethereum(&self) -> AnyResult<EthUnsignedClient> {
        let eth = EthUnsignedClient::new(self.get_url()?).await?;
        Ok(eth)
    }

    /// Create a signed ethers client (used for sending transactions).
    pub async fn get_signed_ethereum(&self) -> AnyResult<EthSignedClient> {
        let eth = self
            .get_unsigned_ethereum()
            .await?
            .sign_with_string(
                self.get_key_string()?.as_str(),
                self.gas_metrics_path.clone(),
            )
            .await?;
        Ok(eth)
    }
}

/// Reusable parameters and helpers for connecting to the Liberland chain.
#[derive(Args, Debug, Clone)]
pub struct LiberlandClient {
    #[clap(from_global)]
    liberland_key: Option<String>,
    #[clap(from_global)]
    liberland_key_file: Option<String>,
    #[clap(from_global)]
    liberland_url: Option<String>,
}

impl LiberlandClient {
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.liberland_key, &self.liberland_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::LiberlandKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    pub fn get_url(&self) -> AnyResult<String> {
        Ok(self
            .liberland_url
            .clone()
            .ok_or(CliError::LiberlandEndpoint)?)
    }

    pub async fn get_unsigned_substrate(&self) -> AnyResult<SubUnsignedClient<LiberlandConfig>> {
        let sub = SubUnsignedClient::new(self.get_url()?).await?;
        Ok(sub)
    }

    pub async fn get_signed_substrate(&self) -> AnyResult<SubSignedClient<LiberlandConfig>> {
        let sub = self
            .get_unsigned_substrate()
            .await?
            .signed(subxt::tx::PairSigner::new(
                KeyPair::from_string(&self.get_key_string()?, None)
                    .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
            ))
            .await?;
        Ok(sub)
    }
}
