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

use std::marker::PhantomData;

use scale_encode::EncodeAsFields;

use crate::{
    types::{BlockNumberOrHash, PalletInfo},
    Error, SubResult,
};

pub struct UnsignedTxs<T: subxt::Config> {
    client: crate::UnsignedClient<T>,
    txs: subxt::tx::TxClient<T, subxt::OnlineClient<T>>,
}

impl<T: subxt::Config> UnsignedTxs<T> {
    pub async fn from_client(client: crate::UnsignedClient<T>) -> SubResult<Self> {
        let client = client.at(BlockNumberOrHash::Best).await?;
        Ok(Self {
            txs: client.inner.tx(),
            client,
        })
    }

    pub fn client(&self) -> crate::UnsignedClient<T> {
        self.client.clone()
    }

    pub fn txs(&self) -> subxt::tx::TxClient<T, subxt::OnlineClient<T>> {
        self.txs.clone()
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct UnsignedTx<CallData> {
    pallet: PalletInfo,
    call: &'static str,
    #[derive_where(skip)]
    _phantom: PhantomData<CallData>,
}

impl<CallData> UnsignedTx<CallData>
where
    CallData: EncodeAsFields + std::fmt::Debug,
{
    pub const fn new(pallet: PalletInfo, call: &'static str) -> Self {
        Self {
            pallet,
            call,
            _phantom: PhantomData,
        }
    }

    pub fn payload(&self, call_data: CallData) -> subxt::tx::DefaultPayload<CallData> {
        subxt::tx::DefaultPayload::new_static(self.pallet.name, self.call, call_data, [0u8; 32])
            .unvalidated()
    }

    pub fn is_supported<T: subxt::Config>(&self, txs: &UnsignedTxs<T>) -> bool {
        let metadata = txs.client().metadata();
        metadata
            .pallet_by_name(self.pallet.name)
            .and_then(|p| p.call_variant_by_name(self.call))
            .is_some()
    }

    #[instrument(skip(txs, call_data), err(level = "warn"))]
    pub async fn submit<T: subxt::Config>(
        &self,
        txs: &UnsignedTxs<T>,
        call_data: CallData,
    ) -> SubResult<()> {
        debug!("Call data: {call_data:?}");
        if self.is_supported(txs) {
            let res = txs
                .txs
                .create_unsigned(&self.payload(call_data))?
                .submit_and_watch()
                .await;
            match res {
                Ok(progress) => txs.client().wait_for_success(progress).await,
                Err(err) => {
                    let err_str = err.to_string();
                    if err_str.contains("Transaction Already Imported")
                        || err_str.contains("Transaction is temporarily banned")
                    {
                        info!("Probably transaction already submitted: {err:?}");
                        Ok(())
                    } else {
                        Err(err.into())
                    }
                }
            }
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }
}
