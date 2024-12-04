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

pub mod contracts;
pub mod types;
pub mod wallet;

use crate::prelude::*;
use num_bigint::BigUint;
use serde::de::DeserializeOwned;
use sp_core::H256;
use toner::{
    contracts::wallet::WalletOpSendMessage,
    tlb::{
        ser::{CellSerialize, CellSerializeExt},
        Cell,
    },
    ton::{
        boc::{BagOfCells, BagOfCellsArgs},
        message::CommonMsgInfo,
        MsgAddress,
    },
};
use types::*;
use url::Url;
use wallet::*;

#[derive(Clone)]
pub struct TonClient {
    client: reqwest::Client,
    base: Url,
}

impl TonClient {
    pub fn new(base: Url, api_key: Option<String>) -> AnyResult<Self> {
        let mut headers = http::HeaderMap::new();
        if let Some(api_key) = api_key {
            headers.insert("X-API-Key", http::HeaderValue::from_str(&api_key)?);
        }
        Ok(Self {
            base: base.join("api/v2/")?,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
        })
    }

    pub async fn post_request<T: DeserializeOwned, B: Serialize>(
        &self,
        method: &str,
        body: &B,
    ) -> AnyResult<T> {
        trace!("Send {} => {}", method, serde_json::to_string(body)?);
        let res = self
            .client
            .post(self.base.join(method)?)
            .json(body)
            .send()
            .await?;
        let bytes = res.bytes().await?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let pretty = serde_json::to_string_pretty(&value)?;
        for (i, line) in pretty.lines().enumerate() {
            trace!("{i}: {line}");
        }
        let body: TonApiResult<T> = serde_json::from_str(&pretty)?;
        if body.ok {
            Ok(body
                .result
                .ok_or(anyhow!("Post request '{method}' don't have result field"))?)
        } else {
            let err = body.error.unwrap_or("Unknown error".into());
            let code = body.code.unwrap_or(-1);
            Err(anyhow!("Post request '{method}' failed [{code}]: {err}"))
        }
    }

    pub async fn get_request<T: DeserializeOwned>(
        &self,
        method: &str,
        query: &[(String, String)],
    ) -> AnyResult<T> {
        let builder = self.client.get(self.base.join(method)?);
        let builder = if query.is_empty() {
            builder
        } else {
            builder.query(query)
        };
        let res = builder.send().await?;
        let bytes = res.bytes().await?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let pretty = serde_json::to_string_pretty(&value)?;
        for (i, line) in pretty.lines().enumerate() {
            trace!("{i}: {line}");
        }
        let body: TonApiResult<T> = serde_json::from_str(&pretty)?;
        if body.ok {
            Ok(body.result.ok_or(anyhow!(
                "Request '{method}({query:?})' don't have result field"
            ))?)
        } else {
            let err = body.error.unwrap_or("Unknown error".into());
            let code = body.code.unwrap_or(-1);
            Err(anyhow!(
                "Request '{method}({query:?})' failed [{code}]: {err}"
            ))
        }
    }

    pub async fn get_transactions(
        &self,
        address: toner::ton::MsgAddress,
        limit: Option<u32>,
        last_tx: Option<TransactionId>,
        to_lt: Option<i64>,
        archival: Option<bool>,
    ) -> AnyResult<Vec<Transaction>> {
        trace!("Get transactions {address:?}, {limit:?}, {last_tx:?}, {to_lt:?}, {archival:?}");
        let mut query = vec![("address".to_string(), address.to_string())];
        if let Some(limit) = limit {
            query.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(last_tx) = last_tx {
            query.push(("lt".to_string(), last_tx.lt.to_string()));
            query.push(("hash".to_string(), hex::encode(last_tx.hash)));
        }
        if let Some(to_lt) = to_lt {
            query.push(("to_lt".to_string(), to_lt.to_string()));
        }
        if let Some(archival) = archival {
            if archival {
                query.push(("archival".to_string(), "true".to_string()));
            }
        }
        self.get_request("getTransactions", &query).await
    }

    pub async fn run_get_method(
        &self,
        address: MsgAddress,
        method: &str,
        stack: Vec<StackEntry>,
        seqno: Option<i64>,
    ) -> AnyResult<RunResult> {
        self.post_request(
            "runGetMethod",
            &RunGetMethod {
                method: method.to_string(),
                address,
                stack,
                seqno,
            },
        )
        .await
    }

    pub async fn send_boc_return_hash(&self, boc: Vec<u8>) -> AnyResult<SendBocResultHash> {
        self.post_request("sendBocReturnHash", &SendBoc { boc })
            .await
    }
}

pub struct SignedTonClient {
    client: TonClient,
    wallet: TonWallet,
}

impl SignedTonClient {
    pub fn new(client: TonClient, wallet: TonWallet) -> Self {
        Self { client, wallet }
    }

    pub async fn seqno(&self) -> AnyResult<u32> {
        let res = self
            .client
            .run_get_method(self.wallet.address(), "seqno", vec![], None)
            .await?;
        if res.exit_code == 0 {
            if let Some(StackEntry::Int(seqno)) = res.stack.first() {
                Ok(seqno.as_u32())
            } else {
                Err(anyhow!("Got wrong nonce stack"))
            }
        } else {
            Err(anyhow!("Wrong contract, failed to fetch nonce"))
        }
    }

    pub async fn submit<C: CellSerialize>(
        &self,
        body: C,
        dst: MsgAddress,
        value: BigUint,
        bounce: bool,
    ) -> AnyResult<H256> {
        let seqno = self.seqno().await?;
        let now = chrono::Utc::now();
        let expire_at = now + chrono::TimeDelta::seconds(120);
        let msgs = vec![WalletOpSendMessage {
            mode: 3,
            message: TonMessage::<Cell, Cell, Cell> {
                info: CommonMsgInfo::transfer(dst, value, bounce),
                init: None,
                body: body.to_cell()?,
            },
        }];
        let msg = self
            .wallet
            .create_external_message(expire_at, seqno, msgs, false)?;
        let msg = msg.to_cell()?;
        let boc = BagOfCells::from_root(msg);
        let msg = toner::tlb::bits::ser::pack_with(
            boc,
            BagOfCellsArgs {
                has_crc32c: true,
                has_idx: false,
            },
        )?
        .as_raw_slice()
        .to_vec();
        let res = self.client.send_boc_return_hash(msg).await?;
        Ok(res.hash.into())
    }
}
