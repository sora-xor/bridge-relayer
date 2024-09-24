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

use toner::{
    tlb::{
        bits::{de::BitReaderExt, integer::ConstU32, ser::BitWriterExt},
        de::CellDeserialize,
        r#as::Ref,
        ser::CellSerialize,
        Cell,
    },
    ton::MsgAddress,
};

/// ## OutboundMessage
/// TLB: `outbound_message#ffc180ad nonce:uint64 message:SoraEncodedCall{data:^cell} source:address = OutboundMessage`
/// Signature: `OutboundMessage{nonce:uint64,message:SoraEncodedCall{data:^cell},source:address}`
pub struct OutboundMessage {
    pub nonce: u64,
    pub message: Cell,
    pub source: MsgAddress,
}

pub const OUTBOUND_MESSAGE_ID: u32 = 0xffc180ad;

impl CellSerialize for OutboundMessage {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder
            .pack(OUTBOUND_MESSAGE_ID)?
            .pack(self.nonce)?
            .store_as::<_, Ref>(&self.message)?
            .pack(self.source)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for OutboundMessage {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<OUTBOUND_MESSAGE_ID>>()?;
        Ok(Self {
            nonce: parser.unpack()?,
            message: parser.parse_as::<_, Ref>()?,
            source: parser.unpack()?,
        })
    }
}

/// ## SendOutboundMessage
/// TLB: `send_outbound_message#432df181 message:SoraEncodedCall{data:^cell} sender:address = SendOutboundMessage`
/// Signature: `SendOutboundMessage{message:SoraEncodedCall{data:^cell},sender:address}`
pub struct SendOutboundMessage {
    pub message: Cell,
    pub sender: MsgAddress,
}

pub const SEND_OUTBOUND_MESSAGE_ID: u32 = 0x432df181;

impl CellSerialize for SendOutboundMessage {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder
            .pack(SEND_OUTBOUND_MESSAGE_ID)?
            .store_as::<_, Ref>(&self.message)?
            .pack(self.sender)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for SendOutboundMessage {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<SEND_OUTBOUND_MESSAGE_ID>>()?;
        Ok(Self {
            message: parser.parse_as::<_, Ref>()?,
            sender: parser.unpack()?,
        })
    }
}

/// ## SendInboundMessage
/// TLB: `send_inbound_message#44b1824c target:address message:^cell = SendInboundMessage`
/// Signature: `SendInboundMessage{target:address,message:^cell}`
pub struct SendInboundMessage {
    pub target: MsgAddress,
    pub message: Cell,
}

pub const SEND_INBOUND_MESSAGE_ID: u32 = 0x44b1824c;

impl CellSerialize for SendInboundMessage {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder
            .pack(SEND_INBOUND_MESSAGE_ID)?
            .pack(self.target)?
            .store_as::<_, Ref>(&self.message)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for SendInboundMessage {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<SEND_INBOUND_MESSAGE_ID>>()?;
        Ok(Self {
            target: parser.unpack()?,
            message: parser.parse_as::<_, Ref>()?,
        })
    }
}

/// ## RegisterApp
/// TLB: `register_app#267f9407 app:address = RegisterApp`
/// Signature: `RegisterApp{app:address}`
pub struct RegisterApp {
    pub app: MsgAddress,
}

pub const REGISTER_APP_ID: u32 = 0x267f9407;

impl CellSerialize for RegisterApp {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder.pack(REGISTER_APP_ID)?.pack(self.app)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for RegisterApp {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<REGISTER_APP_ID>>()?;
        Ok(Self {
            app: parser.unpack()?,
        })
    }
}

/// ## Reset
/// TLB: `reset#554f8589  = Reset`
/// Signature: `Reset{}`
pub struct Reset;

const RESET_ID: u32 = 0x554f8589;

impl CellSerialize for Reset {
    fn store(
        &self,
        builder: &mut toner::tlb::ser::CellBuilder,
    ) -> Result<(), toner::tlb::ser::CellBuilderError> {
        builder.pack(RESET_ID)?;
        Ok(())
    }
}

impl<'de> CellDeserialize<'de> for Reset {
    fn parse(
        parser: &mut toner::tlb::de::CellParser<'de>,
    ) -> Result<Self, toner::tlb::de::CellParserError<'de>> {
        parser.unpack::<ConstU32<RESET_ID>>()?;
        Ok(Self)
    }
}
