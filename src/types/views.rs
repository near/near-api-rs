use std::sync::Arc;

use near_contract_standards::fungible_token::Balance;
use near_primitives::account::FunctionCallPermission;
use near_primitives::serialize::dec_format;
use near_primitives::types::{BlockHeight, Gas, Nonce, ShardId, StorageUsage};
use serde_with::base64::Base64;
use serde_with::serde_as;

// TODO: THIS IS VERY VERY VERY BAD, BUT WE WILL ITERATE ON IT LATER :)
use near_crypto::PublicKey;

use super::CryptoHash;
use crate::prelude::*;

/// The block header info. This is a non-exhaustive list of items that
/// could be present in a block header. More can be added in the future.
///
/// NOTE: For maintainability purposes, some items have been excluded. If required,
/// please submit an issue to [workspaces](https://github.com/near/near-api-rs/issues).
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
#[non_exhaustive]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub epoch_id: CryptoHash,
    pub next_epoch_id: CryptoHash,
    pub hash: CryptoHash,
    pub prev_hash: CryptoHash,
    pub timestamp_nanosec: u64,
    pub random_value: CryptoHash,
    pub gas_price: Balance,
    pub block_ordinal: Option<u64>,
    pub total_supply: Balance,
    pub last_final_block: CryptoHash,
    pub last_ds_final_block: CryptoHash,
    pub next_bp_hash: CryptoHash,
    pub latest_protocol_version: u32,
    pub prev_state_root: CryptoHash,
    pub chunk_receipts_root: CryptoHash,
    pub chunk_headers_root: CryptoHash,
    pub chunk_tx_root: CryptoHash,
    pub outcome_root: CryptoHash,
    pub challenges_root: CryptoHash,
    pub block_merkle_root: CryptoHash,
}

impl From<near_primitives::views::BlockHeaderView> for BlockHeader {
    fn from(value: near_primitives::views::BlockHeaderView) -> Self {
        Self {
            height: value.height,
            epoch_id: value.epoch_id.into(),
            next_epoch_id: value.next_epoch_id.into(),
            hash: value.hash.into(),
            prev_hash: value.prev_hash.into(),
            timestamp_nanosec: value.timestamp_nanosec,
            random_value: value.random_value.into(),
            gas_price: value.gas_price,
            block_ordinal: value.block_ordinal,
            total_supply: value.total_supply,
            last_final_block: value.last_final_block.into(),
            last_ds_final_block: value.last_ds_final_block.into(),
            next_bp_hash: value.next_bp_hash.into(),
            latest_protocol_version: value.latest_protocol_version,
            prev_state_root: value.prev_state_root.into(),
            chunk_receipts_root: value.chunk_receipts_root.into(),
            chunk_headers_root: value.chunk_headers_root.into(),
            chunk_tx_root: value.chunk_tx_root.into(),
            outcome_root: value.outcome_root.into(),
            challenges_root: value.challenges_root.into(),
            block_merkle_root: value.block_merkle_root.into(),
        }
    }
}

/// The header belonging to a [`Chunk`]. This is a non-exhaustive list of
/// members belonging to a Chunk, where newer fields can be added in the future.
///
/// NOTE: For maintainability purposes, some items have been excluded. If required,
/// please submit an issue to [workspaces](https://github.com/near/workspaces-rs/issues).
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
#[non_exhaustive]
pub struct ChunkHeader {
    pub chunk_hash: CryptoHash,
    pub prev_block_hash: CryptoHash,
    pub height_created: BlockHeight,
    pub height_included: BlockHeight,
    pub shard_id: ShardId,
    pub gas_used: Gas,
    pub gas_limit: Gas,
    pub balance_burnt: Balance,

    pub tx_root: CryptoHash,
    pub outcome_root: CryptoHash,
    pub prev_state_root: CryptoHash,
    pub outgoing_receipts_root: CryptoHash,
    pub encoded_merkle_root: CryptoHash,
    pub encoded_length: u64,
}

impl From<near_primitives::views::ChunkHeaderView> for ChunkHeader {
    fn from(value: near_primitives::views::ChunkHeaderView) -> Self {
        Self {
            chunk_hash: value.chunk_hash.into(),
            prev_block_hash: value.prev_block_hash.into(),
            height_created: value.height_created,
            height_included: value.height_included,
            shard_id: value.shard_id,
            gas_used: value.gas_used,
            gas_limit: value.gas_limit,
            balance_burnt: value.balance_burnt,
            tx_root: value.tx_root.into(),
            outcome_root: value.outcome_root.into(),
            prev_state_root: value.prev_state_root.into(),
            outgoing_receipts_root: value.outgoing_receipts_root.into(),
            encoded_merkle_root: value.encoded_merkle_root.into(),
            encoded_length: value.encoded_length,
        }
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct Block {
    pub author: AccountId,
    pub header: BlockHeader,
    pub chunks: Vec<ChunkHeader>,
}

impl From<near_primitives::views::BlockView> for Block {
    fn from(value: near_primitives::views::BlockView) -> Self {
        Self {
            author: value.author,
            header: value.header.into(),
            chunks: value.chunks.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Eq,
    PartialEq,
    Clone,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct Account {
    #[serde(with = "dec_format")]
    pub amount: Balance,
    #[serde(with = "dec_format")]
    pub locked: Balance,
    pub code_hash: CryptoHash,
    pub storage_usage: StorageUsage,
    /// TODO(2271): deprecated.
    #[serde(default)]
    pub storage_paid_at: BlockHeight,
}

impl From<near_primitives::views::AccountView> for Account {
    fn from(value: near_primitives::views::AccountView) -> Self {
        Self {
            amount: value.amount,
            locked: value.locked,
            code_hash: value.code_hash.into(),
            storage_usage: value.storage_usage,
            storage_paid_at: value.storage_paid_at,
        }
    }
}

/// A  of the contract code.
#[serde_as]
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ContractCode {
    #[serde(rename = "code_base64")]
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
    pub hash: CryptoHash,
}

impl From<near_primitives::views::ContractCodeView> for ContractCode {
    fn from(value: near_primitives::views::ContractCodeView) -> Self {
        Self {
            code: value.code,
            hash: value.hash.into(),
        }
    }
}

#[derive(
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
    Debug,
    Eq,
    PartialEq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum AccessKeyPermission {
    FunctionCall {
        #[serde(with = "dec_format")]
        allowance: Option<Balance>,
        receiver_id: String,
        method_names: Vec<String>,
    },
    FullAccess,
}

impl From<AccessKeyPermission> for near_primitives::account::AccessKeyPermission {
    fn from(value: AccessKeyPermission) -> Self {
        match value {
            AccessKeyPermission::FunctionCall {
                allowance,
                receiver_id,
                method_names,
            } => Self::FunctionCall(FunctionCallPermission {
                allowance,
                receiver_id,
                method_names,
            }),
            AccessKeyPermission::FullAccess => Self::FullAccess,
        }
    }
}

impl From<near_primitives::views::AccessKeyPermissionView> for AccessKeyPermission {
    fn from(value: near_primitives::views::AccessKeyPermissionView) -> Self {
        match value {
            near_primitives::views::AccessKeyPermissionView::FunctionCall {
                allowance,
                receiver_id,
                method_names,
            } => Self::FunctionCall {
                allowance,
                receiver_id,
                method_names,
            },
            near_primitives::views::AccessKeyPermissionView::FullAccess => Self::FullAccess,
        }
    }
}

#[serde_as]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ViewStateResult {
    pub values: Vec<StateItem>,
    #[serde_as(as = "Vec<Base64>")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proof: Vec<Arc<[u8]>>,
}

impl From<near_primitives::views::ViewStateResult> for ViewStateResult {
    fn from(value: near_primitives::views::ViewStateResult) -> Self {
        Self {
            values: value.values.into_iter().map(Into::into).collect(),
            proof: value.proof,
        }
    }
}

#[derive(
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
    Debug,
    Eq,
    PartialEq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct AccessKey {
    pub nonce: Nonce,
    pub permission: AccessKeyPermission,
}

impl From<near_primitives::views::AccessKeyView> for AccessKey {
    fn from(value: near_primitives::views::AccessKeyView) -> Self {
        Self {
            nonce: value.nonce,
            permission: value.permission.into(),
        }
    }
}

/// This type is used to mark keys (arrays of bytes) that are queried from store.
///
/// NOTE: Currently, this type is only used in the _client and RPC to be able to transparently
/// pretty-serialize the bytes arrays as base64-encoded strings (see `serialize.rs`).
#[serde_as]
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
    derive_more::Deref,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
pub struct StoreKey(#[serde_as(as = "Base64")] Vec<u8>);

impl From<near_primitives::types::StoreKey> for StoreKey {
    fn from(value: near_primitives::types::StoreKey) -> Self {
        Self(value.into())
    }
}

/// This type is used to mark values returned from store (arrays of bytes).
///
/// NOTE: Currently, this type is only used in the _client and RPC to be able to transparently
/// pretty-serialize the bytes arrays as base64-encoded strings (see `serialize.rs`).
#[serde_as]
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
    derive_more::Deref,
    derive_more::From,
    derive_more::Into,
)]
#[serde(transparent)]
pub struct StoreValue(#[serde_as(as = "Base64")] Vec<u8>);

impl From<near_primitives::types::StoreValue> for StoreValue {
    fn from(value: near_primitives::types::StoreValue) -> Self {
        Self(value.into())
    }
}

/// Item of the state, key and value are serialized in base64 and proof for inclusion of given state item.
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateItem {
    pub key: StoreKey,
    pub value: StoreValue,
}

impl From<near_primitives::views::StateItem> for StateItem {
    fn from(value: near_primitives::views::StateItem) -> Self {
        Self {
            key: value.key.into(),
            value: value.value.into(),
        }
    }
}

#[serde_as]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StateResult {
    pub values: Vec<StateItem>,
    #[serde_as(as = "Vec<Base64>")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proof: Vec<Arc<[u8]>>,
}

impl From<near_primitives::views::ViewStateResult> for StateResult {
    fn from(value: near_primitives::views::ViewStateResult) -> Self {
        Self {
            values: value.values.into_iter().map(Into::into).collect(),
            proof: value.proof,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct CallResult {
    pub result: Vec<u8>,
    pub logs: Vec<String>,
}

impl From<near_primitives::views::CallResult> for CallResult {
    fn from(value: near_primitives::views::CallResult) -> Self {
        Self {
            result: value.result,
            logs: value.logs,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AccessKeyInfo {
    pub public_key: PublicKey, // TODO: replace it
    pub access_key: AccessKey,
}

impl From<near_primitives::views::AccessKeyInfoView> for AccessKeyInfo {
    fn from(value: near_primitives::views::AccessKeyInfoView) -> Self {
        Self {
            public_key: value.public_key,
            access_key: value.access_key.into(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AccessKeyList {
    pub keys: Vec<AccessKeyInfo>,
}

impl From<near_primitives::views::AccessKeyList> for AccessKeyList {
    fn from(value: near_primitives::views::AccessKeyList) -> Self {
        Self {
            keys: value.keys.into_iter().map(Into::into).collect(),
        }
    }
}
