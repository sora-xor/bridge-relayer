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

use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::{base64::Base64, DisplayFromStr, NoneAsEmptyString};
use toner::ton::MsgAddress;

pub type NumberOrString<T> = serde_with::PickFirst<(T, DisplayFromStr)>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TonApiResult<T> {
    pub code: Option<i64>,
    pub error: Option<String>,
    pub ok: bool,
    pub result: Option<T>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionId {
    #[serde_as(as = "NumberOrString<_>")]
    pub lt: i64,
    #[serde_as(as = "Base64")]
    pub hash: [u8; 32],
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct FullAccountState {
    #[serde_as(as = "NumberOrString<_>")]
    pub balance: i64,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
    pub last_transaction_id: TransactionId,
    pub block_id: BlockIdExt,
    #[serde_as(as = "Base64")]
    pub frozen_hash: Vec<u8>,
    #[serde_as(as = "NumberOrString<_>")]
    pub sync_utime: i64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockIdExt {
    #[serde_as(as = "NumberOrString<_>")]
    pub workchain: i32,
    #[serde_as(as = "NumberOrString<_>")]
    pub shard: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub seqno: i32,
    #[serde_as(as = "Base64")]
    pub root_hash: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub file_hash: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccountAddress {
    #[serde_as(as = "DisplayFromStr")]
    pub account_address: MsgAddress,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub address: AccountAddress,
    #[serde_as(as = "NumberOrString<_>")]
    pub utime: i64,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
    pub transaction_id: TransactionId,
    #[serde_as(as = "NumberOrString<_>")]
    pub fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub storage_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub other_fee: i64,
    pub in_msg: Option<Message>,
    pub out_msgs: Vec<Message>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    #[serde_as(as = "DisplayFromStr")]
    pub source: MsgAddress,
    #[serde_as(as = "NoneAsEmptyString")]
    pub destination: Option<MsgAddress>,
    #[serde_as(as = "NumberOrString<_>")]
    pub value: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub fwd_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub ihr_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub created_lt: i64,
    #[serde_as(as = "Base64")]
    pub body_hash: Vec<u8>,
    pub msg_data: MessageData,
    pub message: Option<String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "@type")]
pub enum MessageData {
    #[serde(rename = "msg.dataRaw")]
    Raw {
        #[serde_as(as = "Base64")]
        body: Vec<u8>,
        #[serde_as(as = "Base64")]
        init_state: Vec<u8>,
    },
    #[serde(rename = "msg.dataText")]
    Text {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
    #[serde(rename = "msg.dataDecryptedText")]
    DecryptedText {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
    #[serde(rename = "msg.dataEncryptedText")]
    EncryptedText {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct RunResult {
    #[serde_as(as = "NumberOrString<_>")]
    pub gas_used: i64,
    pub stack: Vec<StackEntry>,
    pub exit_code: i64,
    pub block_id: BlockIdExt,
    pub last_transaction_id: TransactionId,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct RunGetMethod {
    #[serde_as(as = "DisplayFromStr")]
    pub address: MsgAddress,
    pub method: String,
    pub stack: Vec<StackEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub seqno: Option<i64>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SendBoc {
    #[serde_as(as = "Base64")]
    pub boc: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SendBocResultHash {
    #[serde_as(as = "Base64")]
    pub hash: [u8; 32],
}

#[derive(Debug)]
pub enum StackEntry {
    Int(i128),
}

impl<'de> Deserialize<'de> for StackEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, value): (String, serde_json::Value) = Deserialize::deserialize(deserializer)?;

        match (tag.as_str(), value) {
            ("num", serde_json::Value::String(num)) => Ok(Self::Int(
                i128::from_str_radix(num.trim_start_matches("0x"), 16)
                    .map_err(|_| serde::de::Error::custom("wrong integer"))?,
            )),
            _ => Err(serde::de::Error::custom("unexpected variant")),
        }
    }
}

impl Serialize for StackEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_tuple(2)?;
        match self {
            Self::Int(num) => {
                ser.serialize_element("num")?;
                ser.serialize_element(&format!("{:X}", num))?;
                ser.end()
            }
        }
    }
}
