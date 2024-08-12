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
    abi::sudo::SudoCall,
    types::{BlockNumberOrHash, PalletInfo},
    Error, SubResult,
};

pub struct SignedTxs<T: subxt::Config, P: Send + Sync + 'static> {
    client: crate::SignedClient<T, P>,
    txs: subxt::tx::TxClient<T, subxt::OnlineClient<T>>,
}

impl<T: subxt::Config, P> SignedTxs<T, P>
where
    T: subxt::Config,
    P: Send + Sync + Clone + 'static,
{
    pub async fn from_client(client: crate::SignedClient<T, P>) -> SubResult<Self> {
        let client = client.at(BlockNumberOrHash::Best).await?;
        Ok(Self {
            txs: client.inner.tx(),
            client,
        })
    }

    pub fn client(&self) -> crate::SignedClient<T, P> {
        self.client.clone()
    }

    pub fn txs(&self) -> subxt::tx::TxClient<T, subxt::OnlineClient<T>> {
        self.txs.clone()
    }

    pub fn is_supported(&self, pallet: &str, call: &str) -> bool {
        let metadata = self.client().metadata();
        metadata
            .pallet_by_name(pallet)
            .and_then(|p| p.call_variant_by_name(call))
            .is_some()
    }
}

#[derive_where::derive_where(Clone, Copy, Debug)]
pub struct SignedTx<CallData> {
    pallet: PalletInfo,
    call: &'static str,
    _phantom: PhantomData<CallData>,
}

impl<CallData> SignedTx<CallData>
where
    CallData: EncodeAsFields,
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

    pub fn is_supported<T, P>(&self, txs: &SignedTxs<T, P>) -> bool
    where
        T: subxt::Config,
        P: Send + Sync + Clone + 'static,
    {
        txs.is_supported(self.pallet.name, self.call)
    }

    pub async fn submit<T, P>(&self, txs: &SignedTxs<T, P>, call_data: CallData) -> SubResult<()>
    where
        T: subxt::Config,
        P: sp_core::Pair + Clone + Send + Sync + 'static,
        <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
        T::Signature: From<P::Signature>,
        T::AccountId: From<sp_runtime::AccountId32>,
    {
        if self.is_supported(txs) {
            let progress = txs
                .txs
                .sign_and_submit_then_watch_default(&self.payload(call_data), txs.client().signer())
                .await?;
            txs.client().wait_for_success(progress).await
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }

    pub async fn submit_sudo<T, P>(
        &self,
        txs: &SignedTxs<T, P>,
        call_data: CallData,
    ) -> SubResult<()>
    where
        T: subxt::Config,
        P: sp_core::Pair + Clone + Send + Sync + 'static,
        <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
        T::Signature: From<P::Signature>,
        T::AccountId: From<sp_runtime::AccountId32>,
    {
        if self.is_supported(txs) || txs.is_supported(SudoCall::<()>::PALLET, SudoCall::<()>::CALL)
        {
            let progress = txs
                .txs
                .sign_and_submit_then_watch_default(
                    &SudoCall(self.payload(call_data)),
                    txs.client().signer(),
                )
                .await?;
            txs.client().wait_for_success(progress).await
        } else {
            Err(Error::NotSupported(format!("{:?}", self)))
        }
    }
}
