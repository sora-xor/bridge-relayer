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

use std::time::Duration;

use evm_client::alloy::providers::Provider;
use sp_core::H160;
use sub_client::{
    abi::{channel::ChannelSignedTx, evm_app::EvmAppTx},
    bridge_types::EVMChainId,
    sp_runtime::traits::IdentifyAccount,
};
use tracing::Instrument;

use crate::cli::prelude::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    evm: EvmClientCli,
    #[clap(long)]
    channel: H160,
    #[clap(long)]
    app: H160,
    /// Signer for bridge messages
    #[clap(long)]
    seed: String,
    #[clap(long)]
    count: u32,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let sub = self.sub.get_signed_substrate().await?;
        let evm = self.evm.get_evm().await?;
        let chain_id: EVMChainId = evm.chain_id().await?.0.into();
        let mut signers = vec![];
        let mut peers = vec![];
        let mut accounts = vec![];
        for i in 0..self.count {
            let key = format!("//{}//{}", self.seed, i);
            let signer = sp_core::ecdsa::Pair::from_string(&key, None)?;
            let public = signer.public();
            let account = sp_runtime::MultiSigner::Ecdsa(signer.public()).into_account();
            info!(
                account = account.to_string(),
                key,
                public = public.to_string(),
                "Created account {i}"
            );
            peers.push(public);
            signers.push((i, signer));
            accounts.push(public.to_string());
        }
        let addresses = BridgePeers { peers: accounts }.evm_addresses()?;

        let gas_price = evm.unsigned_provider().get_gas_price().await?;
        info!(gas_price, "Gas price");
        info!("Reset channel");
        let channel = evm.signed_channel(self.channel.0.into())?;
        info!("Channel {channel:?}");
        let batch_nonce = channel.batchNonce().with_cloned_provider().await?;
        info!(batch_nonce = batch_nonce.batchNonce, "Batch nonce");
        let call = channel
            .reset(addresses.into_iter().map(|a| a.0.into()).collect())
            .with_cloned_provider();
        debug!(call = call.calldata().to_string(), "Calldata");
        debug!("Static call");
        call.call_raw().await?;
        debug!("Static call");
        call.call().await?;
        debug!("Send");
        let pending = call.send().await?;
        info!("Sent: {}", pending.tx_hash());
        let receipt = pending.get_receipt().await?;
        info!("Reseted: {receipt:?}");

        info!("Register bridge in SORA");
        let tx = sub.tx().await?;
        tokio::try_join!(
            tx.register_signer(chain_id.into(), peers.clone()),
            tx.register_verifier(chain_id.into(), peers.clone()),
            tx.register_evm_channel(chain_id.into(), self.channel),
            tx.register_network(chain_id, self.app, b"BSC".into(), b"BSC".into(), 18),
        )?;
        loop {
            let has_channel = sub
                .storage()
                .await?
                .evm_channel_address(chain_id.into())
                .await?
                .is_some();
            if has_channel {
                break;
            }
            debug!(
                "Waiting for bridge to be available. Channel status = {}",
                has_channel
            );
            tokio::time::sleep(Duration::from_secs(6)).await;
        }

        info!("Start relayers");
        let mut set = tokio::task::JoinSet::new();
        for (i, signer) in signers {
            info!(
                id = i,
                public = signer.public().to_string(),
                "Start relayer"
            );
            let relay = crate::relay::evm::multisig::evm_sub::SubstrateMessagesRelay::new(
                sub.unsigned(),
                evm.clone(),
                signer.clone(),
            )
            .await?;
            set.spawn(relay.run().instrument(tracing::info_span!("relay", id = i)));
            let evm = if i == 0 { evm.clone() } else { evm.unsigned() };
            let relay = crate::relay::evm::multisig::sub_evm::RelayBuilder::new()
                .with_channel_contract(self.channel.0.into())
                .with_signer(Some(signer))
                .with_receiver_client(evm)
                .with_sender_client(sub.unsigned())
                .build()
                .await?;
            set.spawn(relay.run().instrument(tracing::info_span!("relay", id = i)));
        }
        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
