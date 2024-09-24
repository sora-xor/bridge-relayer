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

pub mod abi;
pub mod error;

pub use alloy;
use alloy::{
    network::{Ethereum, EthereumWallet, NetworkWallet},
    primitives::{Address, B256, U256},
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        Identity, Provider, RootProvider,
    },
    sol_types::SolValue,
    transports::BoxTransport,
};
use error::{Error, EvmResult};

pub type UnsignedFiller =
    JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>;

pub type UnsignedProvider =
    FillProvider<UnsignedFiller, RootProvider<BoxTransport>, BoxTransport, Ethereum>;

pub type SignedProvider = FillProvider<
    JoinFill<UnsignedFiller, WalletFiller<EthereumWallet>>,
    RootProvider<BoxTransport>,
    BoxTransport,
    Ethereum,
>;

#[derive(Clone)]
pub struct Client {
    provider: UnsignedProvider,
    wallet: Option<EthereumWallet>,
}

impl Client {
    pub async fn from_url(url: &str) -> EvmResult<Self> {
        let provider = alloy::providers::ProviderBuilder::new()
            .with_recommended_fillers()
            .on_builtin(url)
            .await?;
        Ok(Self {
            provider,
            wallet: None,
        })
    }

    pub fn unsigned_provider(&self) -> UnsignedProvider {
        self.provider.clone()
    }

    pub fn unsigned(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            wallet: None,
        }
    }

    pub fn signed_provider(&self) -> EvmResult<SignedProvider> {
        Ok(self
            .provider
            .clone()
            .join_with(WalletFiller::new(self.wallet()?)))
    }

    pub fn wallet(&self) -> EvmResult<EthereumWallet> {
        self.wallet
            .clone()
            .ok_or(crate::error::Error::UnsignedClient)
    }

    pub fn address(&self) -> EvmResult<Address> {
        Ok(NetworkWallet::<Ethereum>::default_signer_address(
            &self.wallet()?,
        ))
    }

    pub async fn chain_id(&self) -> EvmResult<B256> {
        let chain_id = self.provider.get_chain_id().await?;
        let chain_id = U256::from(chain_id);
        Ok(chain_id.tokenize().0)
    }

    pub fn signed(&self, wallet: EthereumWallet) -> EvmResult<Self> {
        Ok(Self {
            provider: self.provider.clone(),
            wallet: Some(wallet),
        })
    }

    pub fn channel(
        &self,
        channel: Address,
    ) -> abi::Channel::ChannelInstance<BoxTransport, UnsignedProvider> {
        abi::Channel::new(channel, self.unsigned_provider())
    }

    pub fn signed_channel(
        &self,
        channel: Address,
    ) -> EvmResult<abi::Channel::ChannelInstance<BoxTransport, SignedProvider>> {
        Ok(abi::Channel::new(channel, self.signed_provider()?))
    }

    pub async fn token(
        &self,
        token: Address,
    ) -> abi::Token::TokenInstance<BoxTransport, UnsignedProvider> {
        abi::Token::new(token, self.unsigned_provider())
    }

    pub async fn signed_token(
        &self,
        token: Address,
    ) -> EvmResult<abi::Token::TokenInstance<BoxTransport, SignedProvider>> {
        Ok(abi::Token::new(token, self.signed_provider()?))
    }

    pub async fn fungible_app(
        &self,
        app: Address,
    ) -> abi::FAApp::FAAppInstance<BoxTransport, UnsignedProvider> {
        abi::FAApp::new(app, self.unsigned_provider())
    }

    pub async fn signed_fungible_app(
        &self,
        app: Address,
    ) -> EvmResult<abi::FAApp::FAAppInstance<BoxTransport, SignedProvider>> {
        Ok(abi::FAApp::new(app, self.signed_provider()?))
    }

    pub async fn get_finalized_block_number(&self) -> EvmResult<u64> {
        let block = self
            .provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Finalized, true)
            .await?
            .ok_or(Error::BlockNotFound)?;
        block.header.number.ok_or(Error::MissingBlockNumber)
    }
}
