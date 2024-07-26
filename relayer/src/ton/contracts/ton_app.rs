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

use num_bigint::BigInt;
use sp_runtime::AccountId32;
use toner::{
    tlb::{
        bits::{de::BitReaderExt, integer::ConstU32, r#as::NBits, ser::BitWriterExt},
        de::CellDeserialize,
        r#as::FromInto,
        ser::CellSerialize,
    },
    ton::MsgAddress,
};
/// ## SendTon
/// TLB: `send_ton#faeeb2fb soraAddress:Bytes32{data:uint256} amount:int257 = SendTon`
/// Signature: `SendTon{soraAddress:Bytes32{data:uint256},amount:int257}`
pub struct SendTon {
    pub receiver: AccountId32,
    pub amount: BigInt,
}

pub const SEND_TON_ID: u32 = 0xfaeeb2fb;

impl CellSerialize for SendTon {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder
            .pack(SEND_TON_ID)?
            .pack_as::<_, FromInto<[u8; 32]>>(self.receiver.clone())?
            .pack_as::<_, NBits<257>>(self.amount.clone())?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for SendTon {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<SEND_TON_ID>>()?;
        Ok(Self {
            receiver: parser.unpack_as::<_, FromInto<[u8; 32]>>()?,
            amount: parser.unpack_as::<_, NBits<257>>()?,
        })
    }
}

/// ## Migrate
/// TLB: `migrate#0485b71d receiver:address = Migrate`
/// Signature: `Migrate{receiver:address}
pub struct Migrate {
    pub receiver: MsgAddress,
}

pub const MIGRATE_ID: u32 = 0x0485b71d;

impl CellSerialize for Migrate {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder.pack(MIGRATE_ID)?.pack(self.receiver)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for Migrate {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<MIGRATE_ID>>()?;
        Ok(Self {
            receiver: parser.unpack()?,
        })
    }
}
