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

contract Channel {
    uint112 public messageNonce;
    uint112 public batchNonce;
    uint32 public peersCount;

    #[derive(Debug)]
    struct Message {
        address target;
        uint256 max_gas;
        bytes payload;
    }

    #[derive(Debug)]
    struct Batch {
        uint256 nonce;
        uint256 total_max_gas;
        Message[] messages;
    }

    #[derive(Debug)]
    event MessageDispatched(address source, uint256 nonce, bytes payload);

    #[derive(Debug)]
    event BatchDispatched(
        uint256 batch_nonce,
        address relayer,
        uint256 results,
        uint256 results_length,
        uint256 gas_spent,
        uint256 base_fee
    );

    #[derive(Debug)]
    event ChangePeers(address peerId, bool removal);
    
    #[derive(Debug)]
    event Reseted(uint256 peers);

    function submit(
        Batch calldata batch,
        uint8[] calldata v,
        bytes32[] calldata r,
        bytes32[] calldata s
    ) external virtual;

    function reset(address[] calldata initialPeers) external;
}
