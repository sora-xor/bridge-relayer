use crate::prelude::*;
use chrono::{DateTime, Utc};
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
    pub fn from_key(key: &str) -> AnyResult<Self> {
        let (version, mnemonic) = key.split_once(":").ok_or(anyhow!("Wrong key format"))?;
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
            version => return Err(anyhow!("Wrong version {version}")),
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
    ) -> anyhow::Result<TonMessage> {
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
