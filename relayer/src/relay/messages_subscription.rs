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

/// Outbound commitment together with auxiliary digest and simplified MMR proof.
pub struct MessageCommitmentWithProof<S: SenderConfig> {
    pub offchain_data: GenericCommitmentWithBlockOf<S>,
    pub digest: AuxiliaryDigest,
    pub leaf: bridge_common::beefy_types::BeefyMMRLeaf,
    pub proof: bridge_common::simplified_proof::Proof<H256>,
}

/// Load and validate the auxiliary digest for a commitment at a specific block.
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
    let valid_items = count_matching_digest_items(&digest, &network_id, &commitment_hash);
    if valid_items != 1 {
        return Err(anyhow!(
            "Expected digest for commitment not found: {:?}",
            digest
        ));
    }
    Ok(digest)
}

/// Count digest items that exactly match both network id AND commitment hash.
fn count_matching_digest_items(
    digest: &AuxiliaryDigest,
    network_id: &GenericNetworkId,
    commitment_hash: &H256,
) -> usize {
    digest
        .logs
        .iter()
        .filter(|log| {
            let AuxiliaryDigestItem::Commitment(digest_network_id, digest_commitment_hash) = log;
            network_id == digest_network_id && commitment_hash == digest_commitment_hash
        })
        .count()
}

/// Load a commitment by nonce and derive the corresponding simplified MMR proof.
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

/// Find a leaf proof whose leaf_extra digest hash matches the given digest.
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
    use bridge_types::types::AuxiliaryDigestItem;
    use bridge_types::EVMChainId;

    #[test]
    fn test_count_matching_digest_items_logic() {
        let nid_a = GenericNetworkId::EVM(EVMChainId::from([0u8; 32]));
        let nid_b = GenericNetworkId::EVM(EVMChainId::from([1u8; 32]));
        let h1 = H256::repeat_byte(1);
        let h2 = H256::repeat_byte(2);

        let digest = AuxiliaryDigest {
            logs: vec![
                AuxiliaryDigestItem::Commitment(nid_a.clone(), h1),        // matches both for (nid_a,h1)
                AuxiliaryDigestItem::Commitment(nid_b.clone(), h1),        // matches on hash only
                AuxiliaryDigestItem::Commitment(nid_a.clone(), h2),        // matches on network only
                AuxiliaryDigestItem::Commitment(nid_b.clone(), H256::zero()), // matches neither
            ],
        };

        // With AND logic, only the exact (network,hash) pair matches
        let count = super::count_matching_digest_items(&digest, &nid_a, &h1);
        assert_eq!(count, 1);

        // No match: different network and unrelated hash
        let count_none = super::count_matching_digest_items(&digest, &nid_b, &H256::repeat_byte(3));
        assert_eq!(count_none, 0);
    }
}
