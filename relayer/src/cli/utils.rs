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

use std::path::PathBuf;

use super::error::*;
use crate::{prelude::*, substrate::traits::KeyPair};
use bridge_types::ton::TonNetworkId;
use clap::*;
use sp_core::{crypto::Ss58Codec, H160};

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
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.substrate_key, &self.substrate_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::SubstrateKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    pub fn get_url(&self) -> AnyResult<String> {
        Ok(self
            .substrate_url
            .clone()
            .ok_or(CliError::SubstrateEndpoint)?)
    }

    pub async fn get_unsigned_substrate(&self) -> AnyResult<SubUnsignedClient<MainnetConfig>> {
        let sub = SubUnsignedClient::new(self.get_url()?).await?;
        Ok(sub)
    }

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

#[derive(Args, Debug, Clone)]
pub struct EvmClient {
    #[clap(from_global)]
    evm_key: Option<String>,
    #[clap(from_global)]
    evm_key_file: Option<String>,
    #[clap(from_global)]
    evm_url: Option<Url>,
    #[clap(from_global)]
    gas_metrics_path: Option<PathBuf>,
}

impl EvmClient {
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.evm_key, &self.evm_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::EvmKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    pub fn get_url(&self) -> AnyResult<Url> {
        Ok(self.evm_url.clone().ok_or(CliError::EvmEndpoint)?)
    }

    pub async fn get_unsigned_evm(&self) -> AnyResult<EthUnsignedClient> {
        let eth = EthUnsignedClient::new(self.get_url()?).await?;
        Ok(eth)
    }

    pub async fn get_signed_evm(&self) -> AnyResult<EthSignedClient> {
        let eth = self
            .get_unsigned_evm()
            .await?
            .sign_with_string(
                self.get_key_string()?.as_str(),
                self.gas_metrics_path.clone(),
            )
            .await?;
        Ok(eth)
    }

    pub async fn get_evm(&self) -> AnyResult<either::Either<EthUnsignedClient, EthSignedClient>> {
        if self.evm_key.is_none() && self.evm_key_file.is_none() {
            Ok(Either::Left(self.get_unsigned_evm().await?))
        } else {
            Ok(Either::Right(self.get_signed_evm().await?))
        }
    }
}

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

#[derive(Args, Debug, Clone)]
pub struct BridgePeers {
    /// Bridge peers
    #[clap(long)]
    peers: Vec<String>,
}

impl BridgePeers {
    pub fn ecdsa_keys(&self) -> AnyResult<Vec<sp_core::ecdsa::Public>> {
        self.peers.iter().try_fold(
            vec![],
            |mut acc, peer| -> AnyResult<Vec<sp_core::ecdsa::Public>> {
                let pk = sp_core::ecdsa::Public::from_string(peer)?;
                acc.push(pk);
                Ok(acc)
            },
        )
    }

    pub fn evm_addresses(&self) -> AnyResult<Vec<H160>> {
        self.ecdsa_keys()?
            .into_iter()
            .try_fold(vec![], |mut acc, peer| -> AnyResult<Vec<H160>> {
                let pk = secp256k1::PublicKey::parse_compressed(&peer.0)?;
                let address = common::eth::public_key_to_eth_address(&pk);
                acc.push(address);
                Ok(acc)
            })
    }
}

#[derive(Args, Debug, Clone)]
pub struct TonClientCli {
    #[clap(from_global)]
    ton_key: Option<String>,
    #[clap(from_global)]
    ton_key_file: Option<String>,
    #[clap(from_global)]
    ton_url: Option<Url>,
    #[clap(from_global)]
    ton_api_key: Option<String>,
}

impl TonClientCli {
    pub fn get_key_string(&self) -> AnyResult<String> {
        match (&self.ton_key, &self.ton_key_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Err(CliError::TonKey.into()),
            (Some(key), _) => Ok(key.clone()),
            (_, Some(key_file)) => Ok(std::fs::read_to_string(key_file)?),
        }
    }

    pub fn get_url(&self) -> AnyResult<Url> {
        Ok(self.ton_url.clone().ok_or(CliError::TonEndpoint)?)
    }

    pub fn get_unsigned_ton(&self) -> AnyResult<crate::ton::TonClient> {
        let client = crate::ton::TonClient::new(self.get_url()?, self.ton_api_key.clone())?;
        Ok(client)
    }

    pub fn get_signed_ton(&self) -> AnyResult<crate::ton::SignedTonClient> {
        let client = self.get_unsigned_ton()?;
        let wallet = crate::ton::wallet::TonWallet::from_key(&self.get_key_string()?)?;
        Ok(crate::ton::SignedTonClient::new(client, wallet))
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum TonNetworkSelector {
    Mainnet,
    Testnet,
}

impl TonNetworkSelector {
    pub fn network(&self) -> TonNetworkId {
        match self {
            Self::Mainnet => TonNetworkId::Mainnet,
            Self::Testnet => TonNetworkId::Testnet,
        }
    }
}
