use std::{
    io::{Read, Write},
    str::FromStr,
    sync::OnceLock,
};

pub mod actions;
pub mod delegate_action;
pub mod result;

use base64::{Engine, prelude::BASE64_STANDARD};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    AccountId, Action, CryptoHash, Nonce, PublicKey, Signature, errors::DataConversionError,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transaction {
    V0(TransactionV0),
    V1(TransactionV1),
}

impl Transaction {
    pub const fn signer_id(&self) -> &AccountId {
        match self {
            Self::V0(tx) => &tx.signer_id,
            Self::V1(tx) => &tx.signer_id,
        }
    }

    pub const fn receiver_id(&self) -> &AccountId {
        match self {
            Self::V0(tx) => &tx.receiver_id,
            Self::V1(tx) => &tx.receiver_id,
        }
    }

    pub const fn nonce(&self) -> Nonce {
        match self {
            Self::V0(tx) => tx.nonce,
            Self::V1(tx) => tx.nonce,
        }
    }

    pub const fn public_key(&self) -> PublicKey {
        match self {
            Self::V0(tx) => tx.public_key,
            Self::V1(tx) => tx.public_key,
        }
    }

    pub fn actions(&self) -> &[Action] {
        match self {
            Self::V0(tx) => &tx.actions,
            Self::V1(tx) => &tx.actions,
        }
    }

    pub const fn actions_mut(&mut self) -> &mut Vec<Action> {
        match self {
            Self::V0(tx) => &mut tx.actions,
            Self::V1(tx) => &mut tx.actions,
        }
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        let actions = match self {
            Self::V0(tx) => &mut tx.actions,
            Self::V1(tx) => &mut tx.actions,
        };
        std::mem::take(actions)
    }

    pub fn get_hash(&self) -> CryptoHash {
        #[allow(clippy::expect_used)]
        let bytes = borsh::to_vec(&self).expect("Failed to serialize");
        CryptoHash::hash(&bytes)
    }
}

impl BorshSerialize for Transaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        match self {
            Self::V0(tx) => BorshSerialize::serialize(tx, writer)?,
            Self::V1(tx) => {
                BorshSerialize::serialize(&1_u8, writer)?;
                BorshSerialize::serialize(tx, writer)?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for Transaction {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let first = u8::deserialize_reader(reader)?;
        let second = u8::deserialize_reader(reader)?;

        // V0 starts with a little-endian AccountId length (at most 64), so its second byte is 0.
        // V1 starts with the tag 1 followed by the nonzero first byte of its AccountId length.
        if second == 0 {
            let prefix = [first, second];
            let mut reader = prefix.chain(reader);
            return TransactionV0::deserialize_reader(&mut reader).map(Self::V0);
        }

        if first == 1 {
            let prefix = [second];
            let mut reader = prefix.chain(reader);
            return TransactionV1::deserialize_reader(&mut reader).map(Self::V1);
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid transaction version tag: {first}"),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Signature,
    #[borsh(skip)]
    #[serde(skip)]
    hash: OnceLock<CryptoHash>,
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
                block_hash: hash.into(),
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
                block_hash: hash.into(),
                actions: actions
                    .into_iter()
                    .map(Action::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        };

        Ok(Self::new(Signature::from_str(&signature)?, transaction))
    }
}

impl From<SignedTransaction> for near_openapi_types::SignedTransaction {
    fn from(transaction: SignedTransaction) -> Self {
        #[allow(clippy::expect_used)]
        let bytes = borsh::to_vec(&transaction).expect("Failed to serialize");
        Self(BASE64_STANDARD.encode(bytes))
    }
}

impl From<SignedTransaction> for PrepopulateTransaction {
    fn from(mut transaction: SignedTransaction) -> Self {
        Self {
            signer_id: transaction.transaction.signer_id().clone(),
            receiver_id: transaction.transaction.receiver_id().clone(),
            actions: transaction.transaction.take_actions(),
        }
    }
}

impl SignedTransaction {
    pub const fn new(signature: Signature, transaction: Transaction) -> Self {
        Self {
            signature,
            transaction,
            hash: OnceLock::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyType;

    fn transaction_v0() -> TransactionV0 {
        TransactionV0 {
            signer_id: "alice.near".parse().unwrap(),
            public_key: PublicKey::empty(KeyType::ED25519),
            nonce: 1,
            receiver_id: "receiver.near".parse().unwrap(),
            block_hash: CryptoHash::default(),
            actions: vec![],
        }
    }

    #[test]
    fn signed_transaction_v0_borsh_round_trips() {
        let signature = Signature::from_parts(KeyType::ED25519, &[0; 64]).unwrap();
        let signed = SignedTransaction::new(signature, Transaction::V0(transaction_v0()));

        let bytes = borsh::to_vec(&signed).unwrap();
        let deserialized = SignedTransaction::try_from_slice(&bytes).unwrap();

        assert_eq!(deserialized, signed);
    }

    #[test]
    fn transaction_v1_borsh_round_trips() {
        let v0 = transaction_v0();
        let transaction = Transaction::V1(TransactionV1 {
            signer_id: v0.signer_id,
            public_key: v0.public_key,
            nonce: v0.nonce,
            receiver_id: v0.receiver_id,
            block_hash: v0.block_hash,
            actions: v0.actions,
            priority_fee: 1,
        });

        let bytes = borsh::to_vec(&transaction).unwrap();
        let deserialized = Transaction::try_from_slice(&bytes).unwrap();

        assert_eq!(deserialized, transaction);
    }

    #[test]
    fn transaction_rejects_invalid_version_tag() {
        let error = Transaction::try_from_slice(&[2, 3]).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "invalid transaction version tag: 2");
    }
}
