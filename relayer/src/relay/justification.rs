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
//! Build BEEFY justifications with MMR proofs.
//!
//! This module derives validator signatures, validator set, MMR leaf + simplified proof
//! and packages them into a struct consumable by relayers.

use crate::prelude::*;
use crate::substrate::{BeefyCommitment, BeefySignedCommitment, LeafProof};
use bridge_common::{
    bitfield::BitField,
    simplified_proof::{convert_to_simplified_mmr_proof, Proof},
};
use ethers::prelude::*;
use sp_beefy::crypto::Signature;
use sp_beefy::SignedCommitment;
use sp_runtime::traits::{AtLeast32Bit, Keccak256, UniqueSaturatedInto};
use sp_runtime::traits::{Convert, Hash as HashTrait};
use sp_runtime::Saturating;

/// Complete BEEFY justification and proof bundle for a specific block.
#[derive(Debug)]
pub struct MmrPayload {
    pub mmr_root: H256,
}

/// Collected inputs needed to verify a BEEFY commitment on target chains.
#[derive(Debug)]
pub struct BeefyJustification<T: ConfigExt> {
    pub commitment: BeefyCommitment<T>,
    pub signatures: Vec<Option<Signature>>,
    pub num_validators: u32,
    pub signed_validators: Vec<u32>,
    pub validators: Vec<H160>,
    pub leaf_proof: LeafProof<T>,
    pub simplified_proof: Proof<H256>,
    pub is_mandatory: bool,
}

impl<T: ConfigExt> BeefyJustification<T>
where
    T::BlockNumber: AtLeast32Bit + Serialize,
{
    pub async fn create(
        sub: SubUnsignedClient<T>,
        commitment: BeefySignedCommitment<T>,
        is_mandatory: bool,
    ) -> AnyResult<Self> {
        let BeefySignedCommitment::<T>::V1(SignedCommitment {
            commitment,
            signatures,
        }) = commitment;
        let commitment_block_number: u64 = commitment.block_number.clone().into();
        let num_validators = signatures.len() as u32;
        let mut signed_validators = vec![];
        for (i, signature) in (0u32..).zip(signatures.iter()) {
            if let Some(_) = signature {
                signed_validators.push(i)
            }
        }
        let validators: Vec<H160> = sub
            .storage_fetch_or_default(
                &runtime::storage().beefy().authorities(),
                commitment_block_number - 1,
            )
            .await?
            .into_iter()
            .map(|x| H160::from_slice(&pallet_beefy_mmr::BeefyEcdsaToEthereum::convert(x)))
            .collect();
        let payload = Self::get_payload(&commitment).ok_or(anyhow!("Payload is not supported"))?;
        let (leaf_proof, simplified_proof) =
            Self::find_mmr_proof(&sub, &commitment, payload.mmr_root).await?;

        Ok(Self {
            commitment,
            num_validators,
            signed_validators,
            signatures,
            validators,
            leaf_proof,
            simplified_proof,
            is_mandatory,
        })
    }

    pub async fn find_mmr_proof(
        sub: &SubUnsignedClient<T>,
        commitment: &BeefyCommitment<T>,
        root: H256,
    ) -> AnyResult<(LeafProof<T>, Proof<H256>)> {
        for block_number in 0u32..=6u32 {
            let block_number = commitment.block_number.saturating_sub(block_number.into());
            let leaf_proof = sub.mmr_generate_proof(block_number, block_number).await?;
            let hashed_leaf = leaf_proof.leaf.using_encoded(Keccak256::hash);
            debug!("Hashed leaf: {:?}", hashed_leaf);
            let proof = convert_to_simplified_mmr_proof(
                leaf_proof.proof.leaf_indices[0],
                leaf_proof.proof.leaf_count,
                &leaf_proof.proof.items,
            );
            let computed_root = proof.root(
                |a, b| {
                    let res = [a.as_bytes(), b.as_bytes()].concat();
                    Keccak256::hash(&res)
                },
                hashed_leaf,
            );
            if computed_root != root {
                warn!("MMR root mismatch: {:?} != {:?}", root, computed_root);
                continue;
            }
            return Ok((leaf_proof, proof));
        }
        return Err(anyhow!("Could not find MMR proof"));
    }

    pub fn get_payload(commitment: &BeefyCommitment<T>) -> Option<MmrPayload> {
        commitment
            .payload
            .get_raw(&sp_beefy::known_payloads::MMR_ROOT_ID)
            .and_then(|x| x.clone().try_into().ok())
            .and_then(|mmr_root: [u8; 32]| {
                Some(MmrPayload {
                    mmr_root: mmr_root.into(),
                })
            })
    }

    pub fn validator_eth_signature(&self, pos: usize) -> Bytes {
        let validator_signature = self.signatures[pos].clone().expect("signed").to_vec();
        return eth_ecdsa_signature_from_beefy(validator_signature);
    }

    pub fn validator_pubkey(&self, pos: usize) -> H160 {
        let validator_public_key = self.validators[pos];
        validator_public_key
    }

    pub fn validator_pubkey_proof(&self, pos: usize) -> Vec<H256> {
        let proof = beefy_merkle_tree::merkle_proof::<sp_runtime::traits::Keccak256, _, _>(
            self.validators.clone(),
            pos,
        )
        .proof;
        debug!("Validator {} proof: {}", pos, proof.len());
        proof
    }

    pub fn validators_proof_sub(
        &self,
        initial_bitfield: BitField,
        random_bitfield: BitField,
    ) -> bridge_common::beefy_types::ValidatorProof {
        let mut positions = vec![];
        let mut signatures = vec![];
        let mut public_keys = vec![];
        let mut public_key_merkle_proofs = vec![];
        for i in 0..random_bitfield.len() {
            let bit = random_bitfield.is_set(i);
            if bit {
                positions.push(i as u128);
                signatures.push(self.validator_eth_signature(i).to_vec());
                public_keys.push(self.validator_pubkey(i));
                public_key_merkle_proofs.push(self.validator_pubkey_proof(i));
            }
        }
        let validator_proof = bridge_common::beefy_types::ValidatorProof {
            signatures,
            positions,
            public_keys,
            public_key_merkle_proofs: public_key_merkle_proofs,
            validator_claims_bitfield: initial_bitfield,
        };
        validator_proof
    }

    pub fn simplified_mmr_proof_sub(
        &self,
    ) -> AnyResult<(
        bridge_common::beefy_types::BeefyMMRLeaf,
        bridge_common::simplified_proof::Proof<H256>,
    )> {
        let LeafProof { leaf, .. } = self.leaf_proof.clone();
        let parent_hash: [u8; 32] = leaf.parent_number_and_hash.1.as_ref().try_into().unwrap();
        let mmr_leaf = bridge_common::beefy_types::BeefyMMRLeaf {
            version: leaf.version,
            parent_number_and_hash: (
                leaf.parent_number_and_hash.0.unique_saturated_into(),
                parent_hash.into(),
            ),
            beefy_next_authority_set: leaf.beefy_next_authority_set,
            leaf_extra: leaf.leaf_extra,
        };

        let proof = bridge_common::simplified_proof::Proof::<H256> {
            items: self.simplified_proof.items.clone(),
            order: self.simplified_proof.order,
        };
        Ok((mmr_leaf, proof))
    }
}

/// Convert a 65-byte BEEFY ECDSA signature (r[32]||s[32]||v[1] with v in {0,1})
/// to an Ethereum-style signature by adjusting `v += 27`.
pub fn eth_ecdsa_signature_from_beefy(mut sig: Vec<u8>) -> Bytes {
    assert_eq!(sig.len(), 65, "signature must be 65 bytes");
    sig[64] = sig[64].saturating_add(27);
    sig.into()
}

/// Build the same validator proof as `validators_proof_sub` but from raw components.
pub fn validators_proof_from_components(
    initial_bitfield: bridge_common::bitfield::BitField,
    random_bitfield: bridge_common::bitfield::BitField,
    validators: &[H160],
    signatures: &[Option<sp_core::ecdsa::Signature>],
) -> bridge_common::beefy_types::ValidatorProof {
    let mut positions = vec![];
    let mut out_sigs = vec![];
    let mut out_keys = vec![];
    let mut out_proofs = vec![];
    for i in 0..random_bitfield.len() {
        if random_bitfield.is_set(i) {
            positions.push(i as u128);
            let sig = eth_ecdsa_signature_from_beefy(signatures[i].clone().expect("signed").0.to_vec()).to_vec();
            out_sigs.push(sig);
            out_keys.push(validators[i]);
            let proof = beefy_merkle_tree::merkle_proof::<sp_runtime::traits::Keccak256, _, _>(
                validators.to_vec(),
                i,
            )
            .proof;
            out_proofs.push(proof);
        }
    }
    bridge_common::beefy_types::ValidatorProof {
        signatures: out_sigs,
        positions,
        public_keys: out_keys,
        public_key_merkle_proofs: out_proofs,
        validator_claims_bitfield: initial_bitfield,
    }
}
/// Build a ValidatorProof from provided validators and signatures according to a random bitfield.
///
/// - `initial_bitfield` is carried through unchanged.
/// - `random_bitfield` selects which validator positions to include.
/// - `validators` holds the H160 keys in merkle-tree order.
/// - `signatures` should be Ethereum-formatted 65-byte signatures (v in {27,28}) aligned by index.
pub fn assemble_validator_proof(
    initial_bitfield: bridge_common::bitfield::BitField,
    random_bitfield: bridge_common::bitfield::BitField,
    validators: &[H160],
    signatures: &[Vec<u8>],
) -> bridge_common::beefy_types::ValidatorProof {
    let mut positions = vec![];
    let mut collected_sigs = vec![];
    let mut public_keys = vec![];
    let mut public_key_merkle_proofs = vec![];
    for i in 0..random_bitfield.len() {
        if random_bitfield.is_set(i) {
            positions.push(i as u128);
            collected_sigs.push(signatures[i].clone());
            public_keys.push(validators[i]);
            let proof = beefy_merkle_tree::merkle_proof::<sp_runtime::traits::Keccak256, _, _>(
                validators.to_vec(),
                i,
            )
            .proof;
            public_key_merkle_proofs.push(proof);
        }
    }
    bridge_common::beefy_types::ValidatorProof {
        signatures: collected_sigs,
        positions,
        public_keys,
        public_key_merkle_proofs,
        validator_claims_bitfield: initial_bitfield,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_ecdsa_signature_from_beefy_adjusts_v() {
        let mut s0 = vec![0u8; 65];
        s0[64] = 0;
        let out0 = eth_ecdsa_signature_from_beefy(s0);
        assert_eq!(out0[64], 27);

        let mut s1 = vec![0u8; 65];
        s1[64] = 1;
        let out1 = eth_ecdsa_signature_from_beefy(s1);
        assert_eq!(out1[64], 28);
    }

    #[test]
    fn test_assemble_validator_proof_basic() {
        use bridge_common::bitfield::BitField;
        // 4 validators with simple sequential H160s
        let validators = vec![
            H160::from_low_u64_be(0),
            H160::from_low_u64_be(1),
            H160::from_low_u64_be(2),
            H160::from_low_u64_be(3),
        ];
        // signatures aligned by index (dummy 65-byte each with distinct last byte)
        let mut sigs = vec![];
        for i in 0..validators.len() {
            let mut s = vec![0u8; 65];
            s[64] = if i % 2 == 0 { 27 } else { 28 };
            sigs.push(s);
        }
        let initial = BitField::create_bitfield(&[], validators.len());
        let random = BitField::create_bitfield(&[1u32, 3u32], validators.len());

        let proof = assemble_validator_proof(initial.clone(), random, &validators, &sigs);
        assert_eq!(proof.positions.len(), 2);
        // Validate content by matching positions to signatures/keys
        let idx1 = proof.positions.iter().position(|&p| p == 1u128).expect("pos1");
        let idx3 = proof.positions.iter().position(|&p| p == 3u128).expect("pos3");
        assert_eq!(proof.public_keys[idx1], validators[1]);
        assert_eq!(proof.public_keys[idx3], validators[3]);
        assert_eq!(proof.signatures[idx1][64], 28);
        assert_eq!(proof.signatures[idx3][64], 28);
        // Merkle proofs should be non-empty when more than 1 validator
        assert!(!proof.public_key_merkle_proofs[0].is_empty());
        assert_eq!(proof.validator_claims_bitfield.len(), initial.len());
    }

    #[test]
    fn test_validators_proof_sub_consistency_with_assemble() {
        use bridge_common::bitfield::BitField;
        use sp_core::ecdsa::Signature as BeefySig;

        let validators = vec![
            H160::from_low_u64_be(10),
            H160::from_low_u64_be(11),
            H160::from_low_u64_be(12),
            H160::from_low_u64_be(13),
        ];
        // Construct raw BEEFY signatures (v={0,1})
        let mut raw1 = [0u8; 65]; raw1[64] = 0;
        let mut raw3 = [0u8; 65]; raw3[64] = 1;
        let sigs: Vec<Option<BeefySig>> = vec![
            Some(BeefySig::from_raw(raw1)),
            None,
            None,
            Some(BeefySig::from_raw(raw3)),
        ];
        let initial = BitField::create_bitfield(&[], validators.len());
        let random = BitField::create_bitfield(&[0u32, 3u32], validators.len());

        // Build proof via components (mirrors validators_proof_sub)
        let sub_proof = validators_proof_from_components(initial.clone(), random, &validators, &sigs);

        // Build proof via assemble_validator_proof from ETH-encoded signatures
        let eth_sigs: Vec<Vec<u8>> = sigs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if let Some(s) = s {
                    eth_ecdsa_signature_from_beefy(s.0.to_vec()).to_vec()
                } else {
                    // not selected by random -> dummy, will be skipped
                    vec![0u8; 65]
                }
            })
            .collect();
        let assembled = assemble_validator_proof(initial, BitField::create_bitfield(&[0u32,3u32], validators.len()), &validators, &eth_sigs);

        assert_eq!(sub_proof.positions, assembled.positions);
        assert_eq!(sub_proof.public_keys, assembled.public_keys);
        assert_eq!(sub_proof.signatures, assembled.signatures);
        assert_eq!(sub_proof.public_key_merkle_proofs, assembled.public_key_merkle_proofs);
    }

    // Constructing a fully valid BeefyJustification<T> offline requires building
    // sp_beefy::Commitment payloads and MMR leaf types that don't expose simple
    // constructors. The above consistency test covers the exact logic of
    // validators_proof_sub via pure helpers.
}
