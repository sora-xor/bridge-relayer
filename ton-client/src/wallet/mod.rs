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

use crate::error::{Error, TonResult};
use chrono::{DateTime, Utc};
use log::debug;
use toner::contracts::wallet::mnemonic::Mnemonic;
use toner::contracts::wallet::v4r2::V4R2;
use toner::contracts::wallet::{Wallet, WalletOpSendMessage};
pub use toner::ton::message::Message as TonMessage;
use toner::ton::MsgAddress;
use versions::*;

pub mod versions;

pub enum TonWallet {
    V1R1(Wallet<V1R1>),
    V1R2(Wallet<V1R2>),
    V1R3(Wallet<V1R3>),
    V2R1(Wallet<V2R1>),
    V2R2(Wallet<V2R2>),
    V3R1(Wallet<V3R1>),
    V3R2(Wallet<V3R2>),
    V4R1(Wallet<V4R1>),
    V4R2(Wallet<V4R2>),
}

impl TonWallet {
    pub fn from_key(key: &str) -> TonResult<Self> {
        let (version, mnemonic) = key.split_once(":").ok_or(Error::WrongKeyFormat)?;
        let mnemonic: Mnemonic = mnemonic.parse()?;
        let key_pair = mnemonic.generate_keypair(None)?;
        let wallet = match version {
            "v1r1" | "V1R1" => Self::V1R1(Wallet::derive_default(key_pair)?),
            "v1r2" | "V1R2" => Self::V1R2(Wallet::derive_default(key_pair)?),
            "v1r3" | "V1R3" => Self::V1R3(Wallet::derive_default(key_pair)?),
            "v2r1" | "V2R1" => Self::V2R1(Wallet::derive_default(key_pair)?),
            "v2r2" | "V2R2" => Self::V2R2(Wallet::derive_default(key_pair)?),
            "v3r1" | "V3R1" => Self::V3R1(Wallet::derive_default(key_pair)?),
            "v3r2" | "V3R2" => Self::V3R2(Wallet::derive_default(key_pair)?),
            "v4r1" | "V4R1" => Self::V4R1(Wallet::derive_default(key_pair)?),
            "v4r2" | "V4R2" => Self::V4R2(Wallet::derive_default(key_pair)?),
            version => return Err(Error::WrongWalletVersion(version.into())),
        };
        debug!("Initialized wallet: {}", wallet.address());
        Ok(wallet)
    }

    pub fn address(&self) -> MsgAddress {
        match self {
            TonWallet::V1R1(w) => w.address(),
            TonWallet::V1R2(w) => w.address(),
            TonWallet::V1R3(w) => w.address(),
            TonWallet::V2R1(w) => w.address(),
            TonWallet::V2R2(w) => w.address(),
            TonWallet::V3R1(w) => w.address(),
            TonWallet::V3R2(w) => w.address(),
            TonWallet::V4R1(w) => w.address(),
            TonWallet::V4R2(w) => w.address(),
        }
    }

    pub fn wallet_id(&self) -> u32 {
        match self {
            TonWallet::V1R1(w) => w.wallet_id(),
            TonWallet::V1R2(w) => w.wallet_id(),
            TonWallet::V1R3(w) => w.wallet_id(),
            TonWallet::V2R1(w) => w.wallet_id(),
            TonWallet::V2R2(w) => w.wallet_id(),
            TonWallet::V3R1(w) => w.wallet_id(),
            TonWallet::V3R2(w) => w.wallet_id(),
            TonWallet::V4R1(w) => w.wallet_id(),
            TonWallet::V4R2(w) => w.wallet_id(),
        }
    }
    pub fn create_external_message(
        &self,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = WalletOpSendMessage>,
        state_init: bool,
    ) -> TonResult<TonMessage> {
        let message = match self {
            TonWallet::V1R1(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V1R2(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V1R3(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V2R1(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V2R2(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V3R1(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V3R2(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V4R1(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
            TonWallet::V4R2(w) => w
                .create_external_message(expire_at, seqno, msgs, state_init)?
                .normalize()?,
        };
        Ok(message)
    }
}
