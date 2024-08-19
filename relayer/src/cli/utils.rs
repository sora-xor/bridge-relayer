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
use crate::prelude::*;
use clap::*;
use evm_client::alloy::providers::Provider;
use evm_client::alloy::signers::Signer;
use sub_client::bridge_types::ton::TonNetworkId;
use sub_client::sp_core::{crypto::Ss58Codec, H160};
use tracing::Instrument;

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

    #[instrument]
    pub async fn get_unsigned_substrate(&self) -> AnyResult<SubUnsignedClient<SoraConfig>> {
        let sub = SubUnsignedClient::from_url(&self.get_url()?).await?;
        let sub_cloned = sub.clone();
        tokio::spawn(
            async move {
                if let Err(err) = sub_cloned.follow_runtime_upgrades().await {
                    error!("Runtime upgrade subscriber failed: {err:?}, exiting...");
                    std::process::exit(1);
                }
            }
            .instrument(tracing::info_span!("runtime-upgrades")),
        );
        Ok(sub)
    }

    pub async fn get_signed_substrate(
        &self,
    ) -> AnyResult<SubSignedClient<SoraConfig, sub_client::sp_core::sr25519::Pair>> {
        let sub = self.get_unsigned_substrate().await?.signed(
            sr25519::Pair::from_string(&self.get_key_string()?, None)
                .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
        );
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
        let sub = SubUnsignedClient::from_url(&self.get_url()?).await?;
        Ok(sub)
    }

    pub async fn get_signed_substrate(
        &self,
    ) -> AnyResult<SubSignedClient<ParachainConfig, sr25519::Pair>> {
        let sub = self.get_unsigned_substrate().await?.signed(
            sr25519::Pair::from_string(&self.get_key_string()?, None)
                .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
        );
        Ok(sub)
    }
}

#[derive(Args, Debug, Clone)]
pub struct EvmClientCli {
    #[clap(from_global)]
    evm_key: Option<String>,
    #[clap(from_global)]
    evm_key_file: Option<String>,
    #[clap(from_global)]
    evm_url: Option<Url>,
    #[clap(from_global)]
    gas_metrics_path: Option<PathBuf>,
}

impl EvmClientCli {
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

    pub async fn get_unsigned_evm(&self) -> AnyResult<EvmClient> {
        Ok(EvmClient::from_url(self.get_url()?.as_ref()).await?)
    }

    pub async fn get_signed_evm(&self) -> AnyResult<EvmClient> {
        let client = self.get_unsigned_evm().await?;
        let mut signer: alloy::signers::local::PrivateKeySigner = self.get_key_string()?.parse()?;
        let chain_id = client.unsigned_provider().get_chain_id().await?;
        signer.set_chain_id(Some(chain_id));
        Ok(client.signed(alloy::network::EthereumWallet::new(signer))?)
    }

    pub async fn get_evm(&self) -> AnyResult<EvmClient> {
        let client = self.get_unsigned_evm().await?;
        if let Ok(key) = self.get_key_string() {
            let mut signer: alloy::signers::local::PrivateKeySigner = key.parse()?;
            let chain_id = client.unsigned_provider().get_chain_id().await?;
            signer.set_chain_id(Some(chain_id));
            Ok(client.signed(alloy::network::EthereumWallet::new(signer))?)
        } else {
            Ok(client)
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
        let sub = SubUnsignedClient::from_url(&self.get_url()?).await?;
        Ok(sub)
    }

    pub async fn get_signed_substrate(
        &self,
    ) -> AnyResult<SubSignedClient<LiberlandConfig, sr25519::Pair>> {
        let sub = self.get_unsigned_substrate().await?.signed(
            sr25519::Pair::from_string(&self.get_key_string()?, None)
                .map_err(|e| anyhow!("Invalid key: {:?}", e))?,
        );
        Ok(sub)
    }
}

#[derive(Args, Debug, Clone)]
pub struct BridgePeers {
    /// Bridge peers
    #[clap(long)]
    pub peers: Vec<String>,
}

impl BridgePeers {
    pub fn ecdsa_keys(&self) -> AnyResult<Vec<ecdsa::Public>> {
        self.peers
            .iter()
            .try_fold(vec![], |mut acc, peer| -> AnyResult<Vec<ecdsa::Public>> {
                let pk = ecdsa::Public::from_string(peer)?;
                acc.push(pk);
                Ok(acc)
            })
    }

    pub fn evm_addresses(&self) -> AnyResult<Vec<H160>> {
        self.ecdsa_keys()?
            .into_iter()
            .try_fold(vec![], |mut acc, peer| -> AnyResult<Vec<H160>> {
                let address = evm_client::alloy::primitives::Address::from_public_key(
                    &alloy::signers::k256::ecdsa::VerifyingKey::from_sec1_bytes(&peer.0)?,
                );
                acc.push(address.0 .0.into());
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

    pub fn get_unsigned_ton(&self) -> AnyResult<ton_client::TonClient> {
        let client = ton_client::TonClient::new(self.get_url()?, self.ton_api_key.clone())?;
        Ok(client)
    }

    pub fn get_signed_ton(&self) -> AnyResult<ton_client::SignedTonClient> {
        let client = self.get_unsigned_ton()?;
        let wallet = ton_client::wallet::TonWallet::from_key(&self.get_key_string()?)?;
        Ok(ton_client::SignedTonClient::new(client, wallet))
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
