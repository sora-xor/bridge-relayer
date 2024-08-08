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
package main

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/xssnick/tonutils-go/address"
	"github.com/xssnick/tonutils-go/liteclient"
	"github.com/xssnick/tonutils-go/ton"

	"net/http"
)

var api ton.APIClient
var addr address.Address

func main() {
	client := liteclient.NewConnectionPool()

	configUrl := "https://ton-blockchain.github.io/testnet-global.config.json"
	err := client.AddConnectionsFromConfigUrl(context.Background(), configUrl)
	if err != nil {
		panic(err)
	}

	// initialize
	api = *ton.NewAPIClient(client)
	addr = *address.MustParseAddr("EQCAbsiv8R51J3vbdwuDCjz_-8pny_fI-Q-HaeO2WZdkMsKW")

	http.HandleFunc("/nonce", getNonce)
	http.HandleFunc("/messages", getMessages)
	fmt.Println("Server is listening on port 8080...")
	http.ListenAndServe(":8080", nil)
}

type ErrorResponce struct {
	Error string `json:"error"`
}

type NonceResponce struct {
	Nonce uint64 `json:"nonce"`
}

type MessageResponce struct {
	Messages []BridgeMessage `json:"messages"`
}

type BridgeMessage struct {
	Nonce   uint64 `json:"nonce"`
	Sender  string `json:"sender"`
	Payload []byte `json:"payload"`
}

func getNonce(w http.ResponseWriter, r *http.Request) {
	block, err := api.CurrentMasterchainInfo(context.Background())
	if err != nil {
		var errRes = ErrorResponce{Error: "Error Getting Block"}
		var res, err = json.Marshal(errRes)
		if err != nil {
			w.Write([]byte("Marshaling Error"))
			w.WriteHeader(500)
			return
		}
		w.Write(res)
		w.WriteHeader(400)
		return
	}

	res, err := api.RunGetMethod(context.Background(), block, &addr, "outboundNonce")
	if err != nil {
		var errRes = ErrorResponce{Error: "Error Running Get Method"}
		var res, err = json.Marshal(errRes)
		if err != nil {
			w.Write([]byte("Marshaling Error"))
			w.WriteHeader(500)
			return
		}
		w.Write(res)
		w.WriteHeader(400)
		return
	}

	var nonceResponce = NonceResponce{Nonce: res.MustInt(0).Uint64()}

	nonceResBytes, err := json.Marshal(nonceResponce)
	if err != nil {
		w.Write([]byte("Marshaling Error"))
		w.WriteHeader(500)
		return
	}

	w.Write(nonceResBytes)
	w.WriteHeader(200)
}

func getMessages(w http.ResponseWriter, r *http.Request) {
	block, err := api.CurrentMasterchainInfo(context.Background())
	if err != nil {
		var errRes = ErrorResponce{Error: "Error Getting Block"}
		var res, err = json.Marshal(errRes)
		if err != nil {
			w.Write([]byte("Marshaling Error"))
			w.WriteHeader(400)
			return
		}
		w.Write(res)
		w.WriteHeader(400)
		return
	}

	account, err := api.GetAccount(context.Background(), block, &addr)
	if err != nil {
		errRes := ErrorResponce{Error: "Error Getting Account"}
		res, err := json.Marshal(errRes)
		if err != nil {
			w.Write([]byte("Marshaling Error"))
			w.WriteHeader(500)
			return
		}
		w.Write(res)
		w.WriteHeader(400)
		return
	}

	txs, err := api.ListTransactions(context.Background(), &addr, 10, account.LastTxLT, account.LastTxHash)
	if err != nil {
		w.Write([]byte("Get transactions Error"))
		w.WriteHeader(500)
		return
	}

	println(len(txs))

	// messages := make([]BridgeMessage, 0)

	for _, tx := range txs {
		// out := tx.IO.Out //.List.LoadAll()
		// //.List.LoadAll()
		// // if err != nil {
		// // 	println("shit!")
		// // 	continue
		// // }
		// if out == nil {
		// 	// println("shit shit shit!")
		// 	continue
		// }

		// list, err := out.List.LoadAll()
		// if err != nil {
		// 	println("shit!")
		// 	continue
		// }
		// print("list len ")
		// print(i)
		// print(" ")
		// println(len(list))

		cell := tx.IO.In.Msg.Payload() //.Dump()

		// fmt.Println(data)
		if cell.BitsSize() < 32 {
			continue
		}
		parser := cell.BeginParse()
		tbi, err := parser.LoadBigUInt(32)
		if err != nil {
			println("shit type id")
		}
		if tbi.Uint64() != 4290871469 {
			continue
		}
		nbi, err := parser.LoadBigInt(64)
		// message, err := parser
		if err != nil {
			println("shit nonce")
		}

		println(tbi.Uint64())
		println(nbi.Uint64())

		// a := tx.
		// println(a)
		// w.Write([]byte(a))
		// for _, msg := range *tx.IO.Out {
		// 	// a := msg
		// }
	}
	w.Write([]byte("TODO"))
}
