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

use crate::cli::prelude::*;
use bridge_types::H160;

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    eth: EvmClient,
    /// EthApp contract address
    #[clap(long)]
    channel_address: H160,
    #[clap(flatten)]
    peers: BridgePeers,
}

impl Command {
    pub(super) async fn run(&self) -> AnyResult<()> {
        let peers = self.peers.evm_addresses()?;
        info!("Peers: {:?}", peers);
        let eth = self.eth.get_signed_evm().await?;
        let channel = ethereum_gen::ChannelHandler::new(self.channel_address, eth.inner());
        let call = channel.reset(peers);
        info!("Reset {:?}", call.tx.to());
        let call = call.legacy().from(eth.address());
        debug!("Static call: {:?}", call);
        call.call().await?;
        debug!("Send transaction");
        let pending = call.send().await?;
        debug!("Pending transaction: {:?}", pending);
        let result = pending.await?;
        debug!("Confirmed: {:?}", result);
        Ok(())
    }
}
