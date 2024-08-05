use std::sync::Arc;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use toner::{
    contracts::wallet::{v4r2::WalletV4R2Data, WalletOpSendMessage, WalletVersion},
    tlb::{
        bits::ser::BitWriterExt,
        ser::{CellBuilder, CellBuilderError, CellSerialize},
        Cell,
    },
    ton::{boc::BagOfCells, hashmap::HashmapE, UnixTimestamp},
};

lazy_static! {
    static ref WALLET_V4R2_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v4r2.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V4R1_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v4r1.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V3R2_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v3r2.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V3R1_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v3r1.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V2R2_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v2r2.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V2R1_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v2r1.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V1R3_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v1r3.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V1R2_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v1r2.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
    static ref WALLET_V1R1_CODE_CELL: Arc<Cell> = {
        BagOfCells::parse_base64(include_str!("./code/wallet_v1r1.code"))
            .unwrap()
            .single_root()
            .expect("code BoC must be single root")
            .clone()
    };
}

pub struct WalletDataV1 {
    seqno: u32,
    public_key: [u8; 32],
}

impl CellSerialize for WalletDataV1 {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder.pack(self.seqno)?.pack(self.public_key)?;
        Ok(())
    }
}

pub struct WalletDataV3 {
    seqno: u32,
    wallet_id: u32,
    public_key: [u8; 32],
}

impl CellSerialize for WalletDataV3 {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(self.seqno)?
            .pack(self.wallet_id)?
            .pack(self.public_key)?;
        Ok(())
    }
}

pub struct WalletMessageV1 {
    pub wallet_id: u32,
    pub expire_at: DateTime<Utc>,
    pub seqno: u32,
    pub msgs: Vec<WalletOpSendMessage>,
}

impl CellSerialize for WalletMessageV1 {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            .pack(self.wallet_id)?
            .pack_as::<_, UnixTimestamp>(self.expire_at)?
            .pack(self.seqno)?
            .store_many(&self.msgs)?;
        Ok(())
    }
}

pub struct V1R1;

impl WalletVersion for V1R1 {
    type Data = WalletDataV1;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V1R1_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(_wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV1 {
            seqno: 0,
            public_key,
        }
    }
}

pub struct V1R2;

impl WalletVersion for V1R2 {
    type Data = WalletDataV1;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V1R2_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(_wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV1 {
            seqno: 0,
            public_key,
        }
    }
}

pub struct V1R3;

impl WalletVersion for V1R3 {
    type Data = WalletDataV1;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V1R3_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(_wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV1 {
            seqno: 0,
            public_key,
        }
    }
}

pub struct V2R1;

impl WalletVersion for V2R1 {
    type Data = WalletDataV1;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V2R1_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(_wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV1 {
            seqno: 0,
            public_key,
        }
    }
}

pub struct V2R2;

impl WalletVersion for V2R2 {
    type Data = WalletDataV1;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V2R2_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(_wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV1 {
            seqno: 0,
            public_key,
        }
    }
}

pub struct V3R1;

impl WalletVersion for V3R1 {
    type Data = WalletDataV3;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V3R1_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV3 {
            seqno: 0,
            public_key,
            wallet_id,
        }
    }
}

pub struct V3R2;

impl WalletVersion for V3R2 {
    type Data = WalletDataV3;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V3R2_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(wallet_id: u32, public_key: [u8; 32]) -> Self::Data {
        WalletDataV3 {
            seqno: 0,
            public_key,
            wallet_id,
        }
    }
}

pub struct V4R1;

impl WalletVersion for V4R1 {
    type Data = WalletV4R2Data;
    type MessageBody = WalletMessageV1;
    fn code() -> std::sync::Arc<toner::tlb::Cell> {
        WALLET_V4R1_CODE_CELL.clone()
    }

    fn create_external_body(
        wallet_id: u32,
        expire_at: DateTime<Utc>,
        seqno: u32,
        msgs: impl IntoIterator<Item = toner::contracts::wallet::WalletOpSendMessage>,
    ) -> Self::MessageBody {
        WalletMessageV1 {
            wallet_id,
            expire_at,
            seqno,
            msgs: msgs.into_iter().collect(),
        }
    }

    fn init_data(wallet_id: u32, pubkey: [u8; 32]) -> Self::Data {
        WalletV4R2Data {
            seqno: 0,
            pubkey,
            wallet_id,
            plugins: HashmapE::Empty,
        }
    }
}
