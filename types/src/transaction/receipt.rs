use borsh::{BorshDeserialize, BorshSerialize};
use near_gas::NearGas;
use serde::{Deserialize, Serialize};

use crate::{
    AccountId, Action, CryptoHash, PublicKey, ShardId, json::Base64VecU8,
    transaction::actions::GlobalContractIdentifier,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum ReceiptEnum {
    Action(ActionReceipt),
    Data(DataReceipt),
    PromiseYield(ActionReceipt),
    PromiseResume(DataReceipt),
    GlobalContractDistribution(GlobalContractDistributionReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct DataReceiver {
    pub data_id: CryptoHash,
    pub receiver_id: AccountId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct ActionReceipt {
    pub signer_id: AccountId,
    pub signer_public_key: PublicKey,
    pub gas_price: NearGas,
    pub output_data_receivers: Vec<DataReceiver>,
    pub input_data_ids: Vec<CryptoHash>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum GlobalContractDistributionReceipt {
    V1(GlobalContractDistributionReceiptV1),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct GlobalContractDistributionReceiptV1 {
    id: GlobalContractIdentifier,
    target_shard: ShardId,
    already_delivered_shards: Vec<ShardId>,
    #[serde(with = "crate::utils::base64_bytes")]
    code: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct DataReceipt {
    pub data_id: CryptoHash,
    pub data: Option<Base64VecU8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct DelayedReceipt {
    pub index: Option<u64>,
    pub receipt: Box<Receipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum ReceiptPriority {
    Priority(u64),
    NoPriority,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct ReceiptV0 {
    pub predecessor_id: AccountId,
    pub receiver_id: AccountId,
    pub receipt_id: CryptoHash,
    pub receipt: ReceiptEnum,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct ReceiptV1 {
    pub predecessor_id: AccountId,
    pub receiver_id: AccountId,
    pub receipt_id: CryptoHash,
    pub receipt: ReceiptEnum,
    pub priority: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub enum Receipt {
    V0(ReceiptV0),
    V1(ReceiptV1),
}

impl Receipt {
    pub fn receiver_id(&self) -> &AccountId {
        match self {
            Receipt::V0(receipt) => &receipt.receiver_id,
            Receipt::V1(receipt) => &receipt.receiver_id,
        }
    }

    pub fn set_receiver_id(&mut self, receiver_id: AccountId) {
        match self {
            Receipt::V0(receipt) => receipt.receiver_id = receiver_id,
            Receipt::V1(receipt) => receipt.receiver_id = receiver_id,
        }
    }

    pub fn predecessor_id(&self) -> &AccountId {
        match self {
            Receipt::V0(receipt) => &receipt.predecessor_id,
            Receipt::V1(receipt) => &receipt.predecessor_id,
        }
    }

    pub fn set_predecessor_id(&mut self, predecessor_id: AccountId) {
        match self {
            Receipt::V0(receipt) => receipt.predecessor_id = predecessor_id,
            Receipt::V1(receipt) => receipt.predecessor_id = predecessor_id,
        }
    }

    pub fn receipt(&self) -> &ReceiptEnum {
        match self {
            Receipt::V0(receipt) => &receipt.receipt,
            Receipt::V1(receipt) => &receipt.receipt,
        }
    }

    pub fn receipt_mut(&mut self) -> &mut ReceiptEnum {
        match self {
            Receipt::V0(receipt) => &mut receipt.receipt,
            Receipt::V1(receipt) => &mut receipt.receipt,
        }
    }

    pub fn take_receipt(self) -> ReceiptEnum {
        match self {
            Receipt::V0(receipt) => receipt.receipt,
            Receipt::V1(receipt) => receipt.receipt,
        }
    }

    pub fn receipt_id(&self) -> &CryptoHash {
        match self {
            Receipt::V0(receipt) => &receipt.receipt_id,
            Receipt::V1(receipt) => &receipt.receipt_id,
        }
    }

    pub fn set_receipt_id(&mut self, receipt_id: CryptoHash) {
        match self {
            Receipt::V0(receipt) => receipt.receipt_id = receipt_id,
            Receipt::V1(receipt) => receipt.receipt_id = receipt_id,
        }
    }

    pub fn priority(&self) -> ReceiptPriority {
        match self {
            Receipt::V0(_) => ReceiptPriority::NoPriority,
            Receipt::V1(receipt) => ReceiptPriority::Priority(receipt.priority),
        }
    }

    /// It's not a content hash, but receipt_id is unique.
    pub fn get_hash(&self) -> CryptoHash {
        *self.receipt_id()
    }
}
