use std::{cell::OnceCell, io::Write, str::FromStr};

use base64::{Engine, prelude::BASE64_STANDARD};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    AccountId, Action, CryptoHash, Nonce, PublicKey, Signature, errors::DataConversionError, hash,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TransactionV0 {
    pub signer_id: AccountId,
    pub public_key: PublicKey,
    pub nonce: Nonce,
    pub receiver_id: AccountId,
    pub block_hash: CryptoHash,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TransactionV1 {
    pub signer_id: AccountId,
    pub public_key: PublicKey,
    pub nonce: Nonce,
    pub receiver_id: AccountId,
    pub block_hash: CryptoHash,
    pub actions: Vec<Action>,
    pub priority_fee: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub enum Transaction {
    V0(TransactionV0),
    V1(TransactionV1),
}

impl Transaction {
    pub fn signer_id(&self) -> &AccountId {
        match self {
            Transaction::V0(tx) => &tx.signer_id,
            Transaction::V1(tx) => &tx.signer_id,
        }
    }

    pub fn receiver_id(&self) -> &AccountId {
        match self {
            Transaction::V0(tx) => &tx.receiver_id,
            Transaction::V1(tx) => &tx.receiver_id,
        }
    }

    pub fn nonce(&self) -> Nonce {
        match self {
            Transaction::V0(tx) => tx.nonce,
            Transaction::V1(tx) => tx.nonce,
        }
    }

    pub fn public_key(&self) -> &PublicKey {
        match self {
            Transaction::V0(tx) => &tx.public_key,
            Transaction::V1(tx) => &tx.public_key,
        }
    }

    pub fn actions(&self) -> &[Action] {
        match self {
            Transaction::V0(tx) => &tx.actions,
            Transaction::V1(tx) => &tx.actions,
        }
    }

    pub fn actions_mut(&mut self) -> &mut Vec<Action> {
        match self {
            Transaction::V0(tx) => &mut tx.actions,
            Transaction::V1(tx) => &mut tx.actions,
        }
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        let actions = match self {
            Transaction::V0(tx) => &mut tx.actions,
            Transaction::V1(tx) => &mut tx.actions,
        };
        std::mem::take(actions)
    }

    pub fn get_hash(&self) -> CryptoHash {
        let bytes = borsh::to_vec(&self).expect("Failed to deserialize");
        hash(&bytes)
    }
}

impl BorshSerialize for Transaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        match self {
            Transaction::V0(tx) => BorshSerialize::serialize(tx, writer)?,
            Transaction::V1(tx) => {
                BorshSerialize::serialize(&1_u8, writer)?;
                BorshSerialize::serialize(tx, writer)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Signature,
    #[borsh(skip)]
    #[serde(skip)]
    hash: OnceCell<CryptoHash>,
}

impl TryFrom<near_openapi_types::SignedTransactionView> for SignedTransaction {
    type Error = DataConversionError;

    fn try_from(value: near_openapi_types::SignedTransactionView) -> Result<Self, Self::Error> {
        let near_openapi_types::SignedTransactionView {
            signer_id,
            public_key,
            nonce,
            receiver_id,
            actions,
            priority_fee,
            hash,
            signature,
        } = value;

        let transaction = if priority_fee > 0 {
            Transaction::V1(TransactionV1 {
                signer_id,
                public_key: public_key.try_into()?,
                nonce,
                receiver_id,
                block_hash: hash.try_into()?,
                actions: actions
                    .into_iter()
                    .map(Action::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
                priority_fee,
            })
        } else {
            Transaction::V0(TransactionV0 {
                signer_id,
                public_key: public_key.try_into()?,
                nonce,
                receiver_id,
                block_hash: hash.try_into()?,
                actions: actions
                    .into_iter()
                    .map(Action::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        };

        Ok(SignedTransaction::new(
            Signature::from_str(&signature)?,
            transaction,
        ))
    }
}

impl From<SignedTransaction> for near_openapi_types::SignedTransaction {
    fn from(tr: SignedTransaction) -> Self {
        let bytes = borsh::to_vec(&tr).expect("Failed to serialize");
        near_openapi_types::SignedTransaction(BASE64_STANDARD.encode(bytes))
    }
}

impl From<SignedTransaction> for PrepopulateTransaction {
    fn from(mut tr: SignedTransaction) -> Self {
        Self {
            signer_id: tr.transaction.signer_id().clone(),
            receiver_id: tr.transaction.receiver_id().clone(),
            actions: tr.transaction.take_actions(),
        }
    }
}

impl SignedTransaction {
    pub fn new(signature: Signature, transaction: Transaction) -> Self {
        Self {
            signature,
            transaction,
            hash: OnceCell::new(),
        }
    }

    pub fn get_hash(&self) -> CryptoHash {
        *self.hash.get_or_init(|| self.transaction.get_hash())
    }
}

/// An internal type that represents unsigned transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepopulateTransaction {
    /// The account that will sign the transaction.
    pub signer_id: near_account_id::AccountId,
    /// The account that will receive the transaction
    pub receiver_id: near_account_id::AccountId,
    /// The actions that will be executed by the transaction.
    pub actions: Vec<Action>,
}
