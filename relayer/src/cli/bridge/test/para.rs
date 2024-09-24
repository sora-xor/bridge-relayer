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

use sub_client::{bridge_types::SubNetworkId, sp_runtime::traits::IdentifyAccount};
use tracing::Instrument;

use crate::cli::prelude::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    para: ParachainClient,
    /// Signer for bridge messages
    #[clap(long)]
    seed: String,
    #[clap(long)]
    count: u32,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let sub = self.sub.get_signed_substrate().await?;
        let para = self.para.get_signed_substrate().await?;
        let mut signers = vec![];
        let mut peers = vec![];
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
        }

        let mut set = tokio::task::JoinSet::new();

        let tx = sub.tx().await?;
        set.spawn({
            let tx = tx.clone();
            let peers = peers.clone();
            async move { tx.register_signer(SubNetworkId::Rococo.into(), peers).await }
        });
        set.spawn({
            let tx = tx.clone();
            let peers = peers.clone();
            async move {
                tx.register_verifier(SubNetworkId::Rococo.into(), peers)
                    .await
            }
        });

        let tx = para.tx().await?;
        set.spawn({
            let tx = tx.clone();
            let peers = peers.clone();
            async move {
                tx.register_signer(SubNetworkId::Mainnet.into(), peers)
                    .await
            }
        });
        set.spawn({
            let tx = tx.clone();
            let peers = peers.clone();
            async move {
                tx.register_verifier(SubNetworkId::Mainnet.into(), peers)
                    .await
            }
        });

        while let Some(res) = set.join_next().await {
            res??;
        }

        let mut set = tokio::task::JoinSet::new();
        for (i, signer) in signers {
            let para_sora_relay = crate::relay::sub::multisig::RelayBuilder::new()
                .with_receiver_client(
                    sub.unsigned()
                        .with_label((&"para_sora_receiver", &i.to_string()).into()),
                )
                .with_sender_client(
                    para.unsigned()
                        .with_label((&"para_sora_sender", &i.to_string()).into()),
                )
                .with_signer(signer.clone())
                .build()
                .await?;
            let sora_para_relay = crate::relay::sub::multisig::RelayBuilder::new()
                .with_receiver_client(
                    para.unsigned()
                        .with_label((&"sora_para_receiver", &i.to_string()).into()),
                )
                .with_sender_client(
                    sub.unsigned()
                        .with_label((&"sora_para_sender", &i.to_string()).into()),
                )
                .with_signer(signer)
                .build()
                .await?;
            set.spawn(
                sora_para_relay
                    .run()
                    .instrument(tracing::info_span!("sora_para_relay", id = i)),
            );
            set.spawn(
                para_sora_relay
                    .run()
                    .instrument(tracing::info_span!("para_sora_relay", id = i)),
            );
        }
        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
