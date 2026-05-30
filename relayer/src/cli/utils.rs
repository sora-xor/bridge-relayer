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

#[derive(Args, Debug, Clone, Default)]
pub struct BridgeSigner {
    /// Signer for bridge messages
    #[clap(long)]
    signer: Option<String>,
    /// File containing signer for bridge messages
    #[clap(long)]
    signer_file: Option<PathBuf>,
}

impl BridgeSigner {
    fn normalize_key(key: String) -> String {
        key.trim_end_matches(&['\r', '\n'][..]).to_string()
    }

    fn normalize_required_key(key: String) -> AnyResult<String> {
        let key = Self::normalize_key(key);
        if key.trim().is_empty() {
            Err(anyhow!("Bridge signer is empty"))
        } else if key.contains('\0') {
            Err(anyhow!("Bridge signer contains NUL byte"))
        } else {
            Ok(key)
        }
    }

    pub fn get_optional_key_string(&self) -> AnyResult<Option<String>> {
        match (&self.signer, &self.signer_file) {
            (Some(_), Some(_)) => Err(CliError::BothKeyTypesProvided.into()),
            (None, None) => Ok(None),
            (Some(key), _) => Ok(Some(Self::normalize_required_key(key.clone())?)),
            (_, Some(key_file)) => Ok(Some(Self::normalize_required_key(
                std::fs::read_to_string(key_file)?,
            )?)),
        }
    }

    pub fn get_key_string(&self) -> AnyResult<String> {
        self.get_optional_key_string()?
            .ok_or_else(|| anyhow!("Provide bridge signer via --signer or --signer-file"))
    }

    pub fn get_optional_pair(&self) -> AnyResult<Option<sp_core::ecdsa::Pair>> {
        self.get_optional_key_string()?
            .map(|key| {
                sp_core::ecdsa::Pair::from_string(&key, None)
                    .map_err(|e| anyhow!("Invalid signer key: {:?}", e))
            })
            .transpose()
    }

    pub fn get_pair(&self) -> AnyResult<sp_core::ecdsa::Pair> {
        sp_core::ecdsa::Pair::from_string(&self.get_key_string()?, None)
            .map_err(|e| anyhow!("Invalid signer key: {:?}", e))
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

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::Pair as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_signer_file(contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos();
        let sequence = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "bridge-relayer-signer-test-{}-{nonce}-{sequence}.seed",
            std::process::id()
        ));
        std::fs::write(&path, contents).expect("write signer file");
        path
    }

    fn temp_signer_file_bytes(contents: &[u8]) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos();
        let sequence = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "bridge-relayer-signer-test-{}-{nonce}-{sequence}.seed",
            std::process::id()
        ));
        std::fs::write(&path, contents).expect("write signer file");
        path
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let mut directory = std::env::temp_dir();
        directory.push(format!(
            "bridge-relayer-{prefix}-dir-test-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).expect("create temp test directory");
        directory
    }

    #[test]
    fn bridge_signer_file_trims_only_trailing_line_endings() {
        let signer_file = temp_signer_file("  alpha\nbeta  \r\n\n");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        assert_eq!(signer.get_key_string().unwrap(), "  alpha\nbeta  ");

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_inline_trims_trailing_line_endings() {
        let signer = BridgeSigner {
            signer: Some("//Alice\r\n".to_string()),
            signer_file: None,
        };

        assert_eq!(signer.get_key_string().unwrap(), "//Alice");
    }

    #[test]
    fn bridge_signer_rejects_both_inline_and_file_before_reading_file() {
        let signer = BridgeSigner {
            signer: Some("//Alice".to_string()),
            signer_file: Some(PathBuf::from("/path/that/should/not/be/read")),
        };

        let err = signer.get_optional_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
        let err = signer.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
        let err = signer
            .get_optional_pair()
            .err()
            .expect("conflicting signer sources must fail");
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
        let err = signer
            .get_pair()
            .err()
            .expect("conflicting signer sources must fail");
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn bridge_signer_required_key_rejects_missing_sources() {
        let err = BridgeSigner::default().get_key_string().unwrap_err();

        assert!(err.to_string().contains("--signer"));
        assert!(err.to_string().contains("--signer-file"));
    }

    #[test]
    fn bridge_signer_optional_key_allows_missing_sources() {
        assert_eq!(
            BridgeSigner::default().get_optional_key_string().unwrap(),
            None
        );
        assert!(BridgeSigner::default()
            .get_optional_pair()
            .unwrap()
            .is_none());
    }

    #[test]
    fn bridge_signer_file_with_line_ending_matches_inline_pair() {
        let signer_file = temp_signer_file("//Alice\n");
        let file_pair = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        }
        .get_pair()
        .unwrap();
        let inline_pair = BridgeSigner {
            signer: Some("//Alice".to_string()),
            signer_file: None,
        }
        .get_pair()
        .unwrap();

        assert_eq!(file_pair.public(), inline_pair.public());

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_invalid_file_key_is_rejected() {
        let signer_file = temp_signer_file("not a valid signer seed\n");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        let err = signer
            .get_pair()
            .err()
            .expect("invalid signer file must fail");
        assert!(err.to_string().contains("Invalid signer key"));

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_empty_file_key_is_rejected() {
        let signer_file = temp_signer_file("\r\n\n");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        for err in [
            signer.get_optional_key_string().unwrap_err(),
            signer.get_key_string().unwrap_err(),
            signer
                .get_optional_pair()
                .err()
                .expect("empty signer file must fail"),
            signer
                .get_pair()
                .err()
                .expect("empty signer file must fail"),
        ] {
            assert!(err.to_string().contains("Bridge signer is empty"));
        }

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_empty_inline_key_is_rejected() {
        for raw_key in ["", "\n", "\r\n\n", "   \n", "\t\r\n"] {
            let signer = BridgeSigner {
                signer: Some(raw_key.to_string()),
                signer_file: None,
            };

            for err in [
                signer.get_optional_key_string().unwrap_err(),
                signer.get_key_string().unwrap_err(),
                signer
                    .get_optional_pair()
                    .err()
                    .expect("empty inline signer must fail"),
                signer
                    .get_pair()
                    .err()
                    .expect("empty inline signer must fail"),
            ] {
                assert!(err.to_string().contains("Bridge signer is empty"));
            }
        }
    }

    #[test]
    fn bridge_signer_whitespace_only_file_key_is_rejected() {
        let signer_file = temp_signer_file("   \t \r\n");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        for err in [
            signer.get_optional_key_string().unwrap_err(),
            signer.get_key_string().unwrap_err(),
            signer
                .get_optional_pair()
                .err()
                .expect("whitespace-only signer file must fail"),
            signer
                .get_pair()
                .err()
                .expect("whitespace-only signer file must fail"),
        ] {
            assert!(err.to_string().contains("Bridge signer is empty"));
        }

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_missing_file_is_rejected_for_optional_and_required_paths() {
        let mut missing = std::env::temp_dir();
        missing.push(format!(
            "bridge-relayer-missing-signer-test-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(missing),
        };

        assert!(signer.get_optional_key_string().is_err());
        assert!(signer.get_key_string().is_err());
        assert!(signer.get_optional_pair().is_err());
        assert!(signer.get_pair().is_err());
    }

    #[test]
    fn bridge_signer_directory_path_is_rejected() {
        let mut directory = std::env::temp_dir();
        directory.push(format!(
            "bridge-relayer-signer-dir-test-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).expect("create signer directory");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(directory.clone()),
        };

        assert!(signer.get_optional_key_string().is_err());
        assert!(signer.get_key_string().is_err());
        assert!(signer.get_optional_pair().is_err());
        assert!(signer.get_pair().is_err());

        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn bridge_signer_invalid_utf8_file_is_rejected() {
        let signer_file = temp_signer_file_bytes(&[0xff, 0xfe, b'\n']);
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        assert!(signer.get_optional_key_string().is_err());
        assert!(signer.get_key_string().is_err());
        assert!(signer.get_optional_pair().is_err());
        assert!(signer.get_pair().is_err());

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_signer_preserves_trailing_spaces_as_key_material() {
        let signer = BridgeSigner {
            signer: Some("//Alice  \n".to_string()),
            signer_file: None,
        };

        assert_eq!(signer.get_key_string().unwrap(), "//Alice  ");
        assert!(signer.get_pair().is_ok());
    }

    #[test]
    fn bridge_signer_rejects_nul_containing_inline_key() {
        let signer = BridgeSigner {
            signer: Some("//Alice\0//Bob".to_string()),
            signer_file: None,
        };

        for err in [
            signer.get_optional_key_string().unwrap_err(),
            signer.get_key_string().unwrap_err(),
            signer
                .get_optional_pair()
                .err()
                .expect("NUL-containing signer must fail"),
            signer
                .get_pair()
                .err()
                .expect("NUL-containing signer must fail"),
        ] {
            assert!(err.to_string().contains("NUL"));
        }
    }

    #[test]
    fn bridge_signer_rejects_nul_containing_file_key() {
        let signer_file = temp_signer_file("//Alice\0//Bob\n");
        let signer = BridgeSigner {
            signer: None,
            signer_file: Some(signer_file.clone()),
        };

        for err in [
            signer.get_optional_key_string().unwrap_err(),
            signer.get_key_string().unwrap_err(),
            signer
                .get_optional_pair()
                .err()
                .expect("NUL-containing signer file must fail"),
            signer
                .get_pair()
                .err()
                .expect("NUL-containing signer file must fail"),
        ] {
            assert!(err.to_string().contains("NUL"));
        }

        let _ = std::fs::remove_file(signer_file);
    }

    #[test]
    fn bridge_peers_rejects_malformed_public_key() {
        let peers = BridgePeers {
            peers: vec!["not-a-public-key".to_string()],
        };

        assert!(peers.ecdsa_keys().is_err());
        assert!(peers.evm_addresses().is_err());
    }

    #[test]
    fn bridge_peers_rejects_invalid_peer_after_valid_peer() {
        let valid_peer = sp_core::ecdsa::Pair::from_string("//Alice", None)
            .unwrap()
            .public()
            .to_ss58check();
        let peers = BridgePeers {
            peers: vec![valid_peer, "0x1234".to_string()],
        };

        assert!(peers.ecdsa_keys().is_err());
        assert!(peers.evm_addresses().is_err());
    }

    #[test]
    fn bridge_peers_rejects_nul_containing_peer() {
        let peers = BridgePeers {
            peers: vec!["0x1234\0".to_string()],
        };

        assert!(peers.ecdsa_keys().is_err());
        assert!(peers.evm_addresses().is_err());
    }

    #[test]
    fn bridge_peers_rejects_empty_peer_entry() {
        let peers = BridgePeers {
            peers: vec!["".to_string()],
        };

        assert!(peers.ecdsa_keys().is_err());
        assert!(peers.evm_addresses().is_err());
    }

    #[test]
    fn bridge_peers_rejects_whitespace_peer_entry() {
        let peers = BridgePeers {
            peers: vec!["   \t".to_string()],
        };

        assert!(peers.ecdsa_keys().is_err());
        assert!(peers.evm_addresses().is_err());
    }

    #[test]
    fn evm_client_rejects_conflicting_key_sources_before_file_read() {
        let client = EvmClient {
            evm_key: Some("abc".to_string()),
            evm_key_file: Some("/path/that/should/not/be/read".to_string()),
            evm_url: Some(Url::parse("http://localhost:8545").unwrap()),
            gas_metrics_path: None,
        };

        let err = client.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn substrate_client_rejects_conflicting_key_sources_before_file_read() {
        let client = SubstrateClient {
            substrate_key: Some("//Alice".to_string()),
            substrate_key_file: Some("/path/that/should/not/be/read".to_string()),
            substrate_url: Some("ws://localhost:9944".to_string()),
        };

        let err = client.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn parachain_client_rejects_conflicting_key_sources_before_file_read() {
        let client = ParachainClient {
            parachain_key: Some("//Alice".to_string()),
            parachain_key_file: Some("/path/that/should/not/be/read".to_string()),
            parachain_url: Some("ws://localhost:8844".to_string()),
        };

        let err = client.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn liberland_client_rejects_conflicting_key_sources_before_file_read() {
        let client = LiberlandClient {
            liberland_key: Some("//Alice".to_string()),
            liberland_key_file: Some("/path/that/should/not/be/read".to_string()),
            liberland_url: Some("ws://localhost:7744".to_string()),
        };

        let err = client.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn evm_client_rejects_missing_url_and_key() {
        let client = EvmClient {
            evm_key: None,
            evm_key_file: None,
            evm_url: None,
            gas_metrics_path: None,
        };

        assert_eq!(
            client.get_url().unwrap_err().to_string(),
            CliError::EvmEndpoint.to_string()
        );
        assert_eq!(
            client.get_key_string().unwrap_err().to_string(),
            CliError::EvmKey.to_string()
        );
    }

    #[test]
    fn substrate_client_rejects_missing_url_and_key() {
        let client = SubstrateClient {
            substrate_key: None,
            substrate_key_file: None,
            substrate_url: None,
        };

        assert_eq!(
            client.get_url().unwrap_err().to_string(),
            CliError::SubstrateEndpoint.to_string()
        );
        assert_eq!(
            client.get_key_string().unwrap_err().to_string(),
            CliError::SubstrateKey.to_string()
        );
    }

    #[test]
    fn parachain_client_rejects_missing_url_and_key() {
        let client = ParachainClient {
            parachain_key: None,
            parachain_key_file: None,
            parachain_url: None,
        };

        assert_eq!(
            client.get_url().unwrap_err().to_string(),
            CliError::ParachainEndpoint.to_string()
        );
        assert_eq!(
            client.get_key_string().unwrap_err().to_string(),
            CliError::ParachainKey.to_string()
        );
    }

    #[test]
    fn liberland_client_rejects_missing_url_and_key() {
        let client = LiberlandClient {
            liberland_key: None,
            liberland_key_file: None,
            liberland_url: None,
        };

        assert_eq!(
            client.get_url().unwrap_err().to_string(),
            CliError::LiberlandEndpoint.to_string()
        );
        assert_eq!(
            client.get_key_string().unwrap_err().to_string(),
            CliError::LiberlandKey.to_string()
        );
    }

    #[test]
    fn chain_clients_reject_directory_key_files() {
        let substrate_dir = temp_test_dir("substrate-key");
        let parachain_dir = temp_test_dir("parachain-key");
        let liberland_dir = temp_test_dir("liberland-key");
        let evm_dir = temp_test_dir("evm-key");
        let ton_dir = temp_test_dir("ton-key");

        let substrate = SubstrateClient {
            substrate_key: None,
            substrate_key_file: Some(substrate_dir.to_string_lossy().into_owned()),
            substrate_url: Some("ws://localhost:9944".to_string()),
        };
        let parachain = ParachainClient {
            parachain_key: None,
            parachain_key_file: Some(parachain_dir.to_string_lossy().into_owned()),
            parachain_url: Some("ws://localhost:8844".to_string()),
        };
        let liberland = LiberlandClient {
            liberland_key: None,
            liberland_key_file: Some(liberland_dir.to_string_lossy().into_owned()),
            liberland_url: Some("ws://localhost:7744".to_string()),
        };
        let evm = EvmClient {
            evm_key: None,
            evm_key_file: Some(evm_dir.to_string_lossy().into_owned()),
            evm_url: Some(Url::parse("http://localhost:8545").unwrap()),
            gas_metrics_path: None,
        };
        let ton = TonClientCli {
            ton_key: None,
            ton_key_file: Some(ton_dir.to_string_lossy().into_owned()),
            ton_url: Some(Url::parse("https://ton.example/").unwrap()),
            ton_api_key: None,
        };

        assert!(substrate.get_key_string().is_err());
        assert!(parachain.get_key_string().is_err());
        assert!(liberland.get_key_string().is_err());
        assert!(evm.get_key_string().is_err());
        assert!(ton.get_key_string().is_err());

        for directory in [
            substrate_dir,
            parachain_dir,
            liberland_dir,
            evm_dir,
            ton_dir,
        ] {
            let _ = std::fs::remove_dir(directory);
        }
    }

    #[test]
    fn ton_client_cli_rejects_conflicting_key_sources_before_file_read() {
        let client = TonClientCli {
            ton_key: Some("abc".to_string()),
            ton_key_file: Some("/path/that/should/not/be/read".to_string()),
            ton_url: Some(Url::parse("https://ton.example/").unwrap()),
            ton_api_key: None,
        };

        let err = client.get_key_string().unwrap_err();
        assert_eq!(err.to_string(), CliError::BothKeyTypesProvided.to_string());
    }

    #[test]
    fn ton_client_cli_rejects_missing_url_and_key() {
        let client = TonClientCli {
            ton_key: None,
            ton_key_file: None,
            ton_url: None,
            ton_api_key: None,
        };

        assert_eq!(
            client.get_url().unwrap_err().to_string(),
            CliError::TonEndpoint.to_string()
        );
        assert_eq!(
            client.get_key_string().unwrap_err().to_string(),
            CliError::TonKey.to_string()
        );
    }
}
