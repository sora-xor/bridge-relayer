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

use std::collections::BTreeMap;

use crate::prelude::*;
use bridge_types::ton::Commitment;
use bridge_types::{ton::TonNetworkId, GenericNetworkId};
use sp_core::ecdsa;
use sp_runtime::BoundedVec;
use sub_client::abi::channel::{GenericCommitment, MaxU32};
use sub_client::types::BlockNumberOrHash;
use ton_client::{types::StackEntry, TonClient};
use toner::tlb::bits::bitvec::view::AsBits;
use toner::{
    tlb::{bits::de::BitUnpack, de::CellDeserialize},
    ton::MsgAddress,
};

#[derive(Default)]
pub struct RelayBuilder {
    sub: Option<SubUnsignedClient<SoraConfig>>,
    ton: Option<TonClient>,
    signer: Option<ecdsa::Pair>,
    ton_network_id: Option<GenericNetworkId>,
    channel: Option<MsgAddress>,
}

impl RelayBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_signer(mut self, signer: ecdsa::Pair) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_ton_client(mut self, ton_client: TonClient) -> Self {
        self.ton = Some(ton_client);
        self
    }

    pub fn with_channel(mut self, channel: MsgAddress) -> Self {
        self.channel = Some(channel);
        self
    }

    pub fn with_ton_network_id(mut self, ton_network_id: TonNetworkId) -> Self {
        self.ton_network_id = Some(GenericNetworkId::TON(ton_network_id));
        self
    }

    pub fn with_sub_client(mut self, sub: SubUnsignedClient<SoraConfig>) -> Self {
        self.sub = Some(sub);
        self
    }

    pub async fn build(self) -> AnyResult<Relay> {
        let sub = self
            .sub
            .ok_or(anyhow!("Internal error: Substrate client is not provided"))?;
        let sub_network_id = sub.constants().network_id().await?;
        let ton = self
            .ton
            .ok_or(anyhow!("Internal error: TON client is not provided"))?;
        let ton_network_id = self
            .ton_network_id
            .ok_or(anyhow!("Internal error: TON network id is not provided"))?;
        let signer = self
            .signer
            .ok_or(anyhow!("Internal error: Signer is not provided"))?;
        let channel = self
            .channel
            .ok_or(anyhow!("Internal error: Channel address is not provided"))?;
        Ok(Relay {
            sub,
            ton,
            signer,
            sub_network_id,
            ton_network_id,
            channel,
        })
    }
}

pub struct Relay {
    ton: TonClient,
    channel: MsgAddress,
    sub: SubUnsignedClient<SoraConfig>,
    sub_network_id: GenericNetworkId,
    ton_network_id: GenericNetworkId,
    signer: ecdsa::Pair,
}

impl Relay {
    pub async fn ton_nonce(&self) -> AnyResult<u64> {
        let res = self
            .ton
            .run_get_method(self.channel, "outboundNonce", vec![], None)
            .await?;
        if res.exit_code == 0 {
            if let Some(StackEntry::Int(nonce)) = res.stack.first() {
                Ok(*nonce as u64)
            } else {
                Err(anyhow!("Got wrong nonce stack"))
            }
        } else {
            Err(anyhow!("Wrong contract, failed to fetch nonce"))
        }
    }

    async fn messages(&self) -> AnyResult<Vec<Commitment<MaxU32>>> {
        let mut messages = vec![];
        let res = self
            .ton
            .get_transactions(self.channel, None, None, None, Some(true))
            .await?;
        for tx in res {
            for msg in tx.out_msgs {
                if let ton_client::types::MessageData::Raw { body, .. } = msg.msg_data {
                    let body = toner::ton::boc::BagOfCells::unpack(body.as_bits())?
                        .single_root()
                        .cloned()
                        .ok_or(anyhow!("Wrong BoC"))?;
                    match ton_client::contracts::channel::OutboundMessage::parse(&mut body.parser())
                    {
                        Ok(message) => {
                            messages.push(Commitment::Inbound(
                                bridge_types::ton::InboundCommitment {
                                    nonce: message.nonce,
                                    source: bridge_types::ton::TonAddress::new(
                                        message.source.workchain_id as i8,
                                        message.source.address.into(),
                                    ),
                                    channel: bridge_types::ton::TonAddress::new(
                                        msg.source.workchain_id as i8,
                                        msg.source.address.into(),
                                    ),
                                    transaction_id: bridge_types::ton::TonTransactionId {
                                        lt: tx.transaction_id.lt,
                                        hash: tx.transaction_id.hash.into(),
                                    },
                                    payload: BoundedVec::truncate_from(
                                        message.message.data.as_raw_slice().to_vec(),
                                    ),
                                },
                            ));
                        }
                        Err(err) => {
                            warn!("Failed to parse body: {err:?}");
                        }
                    }
                }
            }
        }
        Ok(messages)
    }

    pub async fn sub_nonce(&self) -> AnyResult<u64> {
        let sub_nonce = self
            .sub
            .at(BlockNumberOrHash::Best)
            .await?
            .storage()
            .await?
            .inbound_nonce(self.ton_network_id)
            .await?;
        Ok(sub_nonce)
    }

    async fn send(&self, commitment: Commitment<MaxU32>) -> AnyResult<()> {
        let commitment = GenericCommitment::TON(commitment);
        self.sub
            .submit_inbound_commitment(
                self.signer.clone(),
                self.ton_network_id,
                self.sub_network_id,
                commitment,
            )
            .await?;
        Ok(())
    }

    #[instrument(skip(self), name = "ton_multisig_ton_sub")]
    pub async fn run(self) -> AnyResult<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let mut sub_nonce = self.sub_nonce().await?;
            let ton_nonce = self.ton_nonce().await?;
            info!("Nonces - TON: {}, SORA: {}", ton_nonce, sub_nonce);
            if ton_nonce > sub_nonce {
                let mut found_messages = BTreeMap::new();
                for message in self.messages().await? {
                    found_messages.insert(message.nonce(), message);
                }
                while sub_nonce < ton_nonce {
                    sub_nonce += 1;
                    let message = found_messages.remove(&sub_nonce).ok_or(anyhow!(
                        "Internal error: Message with nonce {sub_nonce} not found"
                    ))?;
                    self.send(message).await?;
                }
            }
        }
    }
}
