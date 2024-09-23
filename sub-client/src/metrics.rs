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

use metrics::Label;
use subxt::backend::rpc::RpcClientT;

pub const SUB_TOTAL_RPC_REQUESTS: &str = "sub_total_rpc_requests";
pub const SUB_TOTAL_SUBSCRIPTIONS: &str = "sub_total_subscriptions";

pub fn describe_metrics() {
    metrics::describe_counter!(SUB_TOTAL_RPC_REQUESTS, "Total Substrate RPC requests");

    metrics::describe_counter!(SUB_TOTAL_SUBSCRIPTIONS, "Total Substrate RPC subscriptions");
}

pub struct RpcClientMetrics(pub jsonrpsee::core::client::Client, pub Vec<Label>);

impl RpcClientT for RpcClientMetrics {
    fn request_raw<'a>(
        &'a self,
        method: &'a str,
        params: Option<Box<jsonrpsee::core::JsonRawValue>>,
    ) -> subxt::backend::rpc::RawRpcFuture<'a, Box<jsonrpsee::core::JsonRawValue>> {
        let counter = metrics::counter!(SUB_TOTAL_RPC_REQUESTS, self.1.clone());
        counter.increment(1);
        self.0.request_raw(method, params)
    }

    fn subscribe_raw<'a>(
        &'a self,
        sub: &'a str,
        params: Option<Box<jsonrpsee::core::JsonRawValue>>,
        unsub: &'a str,
    ) -> subxt::backend::rpc::RawRpcFuture<'a, subxt::backend::rpc::RawRpcSubscription> {
        let counter = metrics::counter!(SUB_TOTAL_SUBSCRIPTIONS, self.1.clone());
        counter.increment(1);
        self.0.subscribe_raw(sub, params, unsub)
    }
}
