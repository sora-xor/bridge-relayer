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

// TODO #167: fix clippy warnings
#![allow(clippy::all)]

use bridge_types::types::{AuxiliaryDigest, AuxiliaryDigestItem};
use bridge_types::{GenericNetworkId, H256};

use crate::substrate::{BlockNumberOrHash, GenericCommitmentWithBlockOf, LeafProof};
use crate::{prelude::*, substrate::BlockNumber};
use bridge_common::simplified_proof::convert_to_simplified_mmr_proof;
use sp_runtime::traits::{Keccak256, UniqueSaturatedInto};

pub struct MessageCommitmentWithProof<S: SenderConfig> {
    pub offchain_data: GenericCommitmentWithBlockOf<S>,
    pub digest: AuxiliaryDigest,
    pub leaf: bridge_common::beefy_types::BeefyMMRLeaf,
    pub proof: bridge_common::simplified_proof::Proof<H256>,
}

fn digest_item_matches(
    log: &AuxiliaryDigestItem,
    network_id: GenericNetworkId,
    commitment_hash: H256,
) -> bool {
    matches!(
        log,
        AuxiliaryDigestItem::Commitment(digest_network_id, digest_commitment_hash)
            if *digest_network_id == network_id && *digest_commitment_hash == commitment_hash
    )
}

fn matching_digest_items(
    digest: &AuxiliaryDigest,
    network_id: GenericNetworkId,
    commitment_hash: H256,
) -> usize {
    digest
        .logs
        .iter()
        .filter(|log| digest_item_matches(log, network_id, commitment_hash))
        .count()
}

pub async fn load_digest<S: SenderConfig>(
    sender: &SubUnsignedClient<S>,
    network_id: GenericNetworkId,
    block_number: BlockNumber<S>,
    commitment_hash: H256,
) -> AnyResult<AuxiliaryDigest> {
    let block_hash = sender.block_hash(block_number).await?;
    let digest = sender.auxiliary_digest(Some(block_hash)).await?;
    if digest.logs.is_empty() {
        return Err(anyhow!("Digest is empty"));
    }
    let valid_items = matching_digest_items(&digest, network_id, commitment_hash);
    if valid_items != 1 {
        return Err(anyhow!(
            "Expected digest for commitment not found: {:?}",
            digest
        ));
    }
    Ok(digest)
}

pub async fn load_commitment_with_proof<S: SenderConfig>(
    sender: &SubUnsignedClient<S>,
    network_id: GenericNetworkId,
    batch_nonce: u64,
    latest_beefy_block: u32,
) -> AnyResult<MessageCommitmentWithProof<S>> {
    let offchain_data = sender
        .commitment_with_nonce(network_id, batch_nonce, BlockNumberOrHash::Finalized)
        .await?;
    let digest = load_digest(
        sender,
        network_id,
        offchain_data.block_number,
        offchain_data.commitment.hash(),
    )
    .await?;
    let digest_hash = Keccak256::hash_of(&digest);
    trace!("Digest hash: {}", digest_hash);
    let leaf_proof = leaf_proof_with_digest(
        sender,
        digest_hash,
        offchain_data.block_number,
        50,
        latest_beefy_block.into(),
    )
    .await?;
    let leaf = leaf_proof.leaf;
    let proof = leaf_proof.proof;
    let parent_hash: [u8; 32] = leaf.parent_number_and_hash.1.as_ref().try_into().unwrap();
    let ready_leaf = bridge_common::beefy_types::BeefyMMRLeaf {
        version: leaf.version,
        parent_number_and_hash: (
            leaf.parent_number_and_hash.0.unique_saturated_into(),
            parent_hash.into(),
        ),
        beefy_next_authority_set: leaf.beefy_next_authority_set,
        leaf_extra: leaf.leaf_extra,
    };
    trace!("Leaf: {:?}", ready_leaf);

    let proof =
        convert_to_simplified_mmr_proof(proof.leaf_indices[0], proof.leaf_count, &proof.items);

    Ok(MessageCommitmentWithProof {
        offchain_data,
        digest,
        leaf: ready_leaf,
        proof,
    })
}

async fn leaf_proof_with_digest<S: SenderConfig>(
    sender: &SubUnsignedClient<S>,
    digest_hash: H256,
    start_leaf: BlockNumber<S>,
    count: u32,
    at: BlockNumber<S>,
) -> AnyResult<LeafProof<S>> {
    for i in 0..count {
        let leaf = start_leaf + i.into();
        let leaf_proof = sender.mmr_generate_proof(leaf, at).await?;
        if leaf_proof.leaf.leaf_extra.digest_hash == digest_hash {
            return Ok(leaf_proof);
        }
    }
    return Err(anyhow::anyhow!("leaf proof not found"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_types::ton::TonNetworkId;
    use bridge_types::SubNetworkId;

    #[test]
    fn digest_item_matches_only_when_network_and_hash_match() {
        let network_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let commitment_hash = H256::repeat_byte(1);
        let different_network = GenericNetworkId::EVM(H256::repeat_byte(20));
        let different_hash = H256::repeat_byte(2);

        assert!(digest_item_matches(
            &AuxiliaryDigestItem::Commitment(network_id, commitment_hash),
            network_id,
            commitment_hash
        ));
        assert!(!digest_item_matches(
            &AuxiliaryDigestItem::Commitment(different_network, commitment_hash),
            network_id,
            commitment_hash
        ));
        assert!(!digest_item_matches(
            &AuxiliaryDigestItem::Commitment(network_id, different_hash),
            network_id,
            commitment_hash
        ));
        assert!(!digest_item_matches(
            &AuxiliaryDigestItem::Commitment(different_network, different_hash),
            network_id,
            commitment_hash
        ));
    }

    #[test]
    fn digest_match_count_ignores_partial_matches_and_counts_duplicates() {
        let network_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let commitment_hash = H256::repeat_byte(1);
        let digest = AuxiliaryDigest {
            logs: vec![
                AuxiliaryDigestItem::Commitment(network_id, commitment_hash),
                AuxiliaryDigestItem::Commitment(network_id, H256::repeat_byte(2)),
                AuxiliaryDigestItem::Commitment(
                    GenericNetworkId::EVM(H256::repeat_byte(20)),
                    commitment_hash,
                ),
                AuxiliaryDigestItem::Commitment(network_id, commitment_hash),
            ],
        };

        assert_eq!(
            matching_digest_items(&digest, network_id, commitment_hash),
            2
        );
    }

    #[test]
    fn digest_match_count_rejects_cross_network_variants_with_same_hash() {
        let network_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let commitment_hash = H256::repeat_byte(1);
        let digest = AuxiliaryDigest {
            logs: vec![
                AuxiliaryDigestItem::Commitment(
                    GenericNetworkId::Sub(SubNetworkId::Mainnet),
                    commitment_hash,
                ),
                AuxiliaryDigestItem::Commitment(GenericNetworkId::EVMLegacy(10), commitment_hash),
                AuxiliaryDigestItem::Commitment(
                    GenericNetworkId::TON(TonNetworkId::Mainnet),
                    commitment_hash,
                ),
            ],
        };

        assert_eq!(
            matching_digest_items(&digest, network_id, commitment_hash),
            0
        );
    }

    #[test]
    fn digest_match_count_is_zero_for_empty_digest() {
        let network_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let commitment_hash = H256::repeat_byte(1);
        let digest = AuxiliaryDigest { logs: vec![] };

        assert_eq!(
            matching_digest_items(&digest, network_id, commitment_hash),
            0
        );
    }

    #[test]
    fn digest_match_count_is_exactly_one_when_noise_surrounds_single_match() {
        let network_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let commitment_hash = H256::repeat_byte(1);
        let digest = AuxiliaryDigest {
            logs: vec![
                AuxiliaryDigestItem::Commitment(network_id, H256::repeat_byte(0)),
                AuxiliaryDigestItem::Commitment(GenericNetworkId::EVMLegacy(10), commitment_hash),
                AuxiliaryDigestItem::Commitment(network_id, commitment_hash),
                AuxiliaryDigestItem::Commitment(
                    GenericNetworkId::Sub(SubNetworkId::Liberland),
                    H256::repeat_byte(0),
                ),
            ],
        };

        assert_eq!(
            matching_digest_items(&digest, network_id, commitment_hash),
            1
        );
    }

    #[test]
    fn digest_item_does_not_coerce_between_evm_and_legacy_evm_ids() {
        let commitment_hash = H256::repeat_byte(1);
        let evm_id = GenericNetworkId::EVM(H256::repeat_byte(10));
        let legacy_id = GenericNetworkId::EVMLegacy(10);

        assert!(!digest_item_matches(
            &AuxiliaryDigestItem::Commitment(evm_id, commitment_hash),
            legacy_id,
            commitment_hash
        ));
        assert!(!digest_item_matches(
            &AuxiliaryDigestItem::Commitment(legacy_id, commitment_hash),
            evm_id,
            commitment_hash
        ));
    }

    #[test]
    fn digest_match_count_rejects_same_hash_across_ton_networks() {
        let commitment_hash = H256::repeat_byte(1);
        let digest = AuxiliaryDigest {
            logs: vec![AuxiliaryDigestItem::Commitment(
                GenericNetworkId::TON(TonNetworkId::Mainnet),
                commitment_hash,
            )],
        };

        assert_eq!(
            matching_digest_items(
                &digest,
                GenericNetworkId::TON(TonNetworkId::Testnet),
                commitment_hash,
            ),
            0
        );
    }
}
