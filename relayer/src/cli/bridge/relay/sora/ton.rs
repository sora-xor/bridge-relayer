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

use num_bigint::BigUint;
use toner::ton::MsgAddress;

use crate::cli::prelude::*;

/// Relay Substrate (SORA) outbound messages to TON inbound channel.
///
/// Reads outbound commitments from SORA, builds TON `SendInboundMessage` cells,
/// and submits via the configured TON wallet. The optional `--value` attaches
/// nanotons per message (default ≈0.1 TON). Use `--no-bounce` to disable bounce.
#[derive(Args, Clone, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    sub: SubstrateClient,
    #[clap(flatten)]
    ton: TonClientCli,
    /// Optional TON value (nanotons) to attach per message (for gas/fees). Defaults to 0.1 TON.
    #[clap(long)]
    value: Option<u64>,
    /// Disable bounce. Defaults to true (bounce enabled).
    #[clap(long)]
    no_bounce: bool,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let ton_signed = self.ton.get_signed_ton()?;
        let ton_unsigned = self.ton.get_unsigned_ton()?;
        let sub = self.sub.get_unsigned_substrate().await?;
        // Determine TON network id from registered app info and read channel address.
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
        let relay = crate::relay::ton::sub_messages::RelayBuilder::new()
            .with_sub_client(sub)
            .with_signed_ton(ton_signed)
            .with_unsigned_ton(ton_unsigned)
            .with_channel(MsgAddress {
                workchain_id: channel_address.workchain.into(),
                address: channel_address.address.0,
            })
            .with_ton_network_id(network_id)
            .with_value(self.value.map(BigUint::from).unwrap_or_else(|| BigUint::from(100_000_000u64)))
            .with_bounce(!self.no_bounce)
            .build()
            .await?;
        relay.run().await?;
        Ok(())
    }
}
