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

use toner::ton::MsgAddress;

use crate::cli::prelude::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    ton: TonClientCli,
    /// Signer for bridge messages
    #[clap(long)]
    signer: String,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let ton = self.ton.get_unsigned_ton()?;
        let sub = self.sub.get_unsigned_substrate().await?;
        let signer = sp_core::ecdsa::Pair::from_string(&self.signer, None)?;
        let Some((network_id, _app)) = sub
            .storage_fetch(&runtime::storage().jetton_app().app_info(), ())
            .await?
        else {
            return Err(anyhow!("Bridge app not registered"));
        };
        let Some(channel_address) = sub
            .storage_fetch(
                &runtime::storage()
                    .bridge_inbound_channel()
                    .ton_channel_addresses(network_id),
                (),
            )
            .await?
        else {
            return Err(anyhow!("Bridge channel not registered"));
        };
        let relay = crate::relay::ton::ton_messages::RelayBuilder::new()
            .with_sub_client(sub)
            .with_ton_client(ton)
            .with_channel(MsgAddress {
                workchain_id: channel_address.workchain.into(),
                address: channel_address.address.0,
            })
            .with_ton_network_id(network_id)
            .with_signer(signer)
            .build()
            .await?;
        relay.run().await?;
        Ok(())
    }
}
