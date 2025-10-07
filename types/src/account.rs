use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{AccountId, CryptoHash, NearToken, StorageUsage, errors::DataConversionError};

#[derive(
    Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq, Default,
)]
pub enum ContractState {
    GlobalHash(CryptoHash),
    GlobalAccountId(AccountId),
    LocalHash(CryptoHash),
    #[default]
    None,
}

impl ContractState {
    pub const fn from_global_contract_hash(hash: CryptoHash) -> Self {
        Self::GlobalHash(hash)
    }

    pub const fn from_local_hash(hash: CryptoHash) -> Self {
        Self::LocalHash(hash)
    }
}

impl From<AccountId> for ContractState {
    fn from(value: AccountId) -> Self {
        Self::GlobalAccountId(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Account {
    pub amount: NearToken,
    pub contract_state: ContractState,
    pub locked: NearToken,
    pub storage_usage: StorageUsage,
}

impl TryFrom<near_openapi_types::AccountView> for Account {
    type Error = DataConversionError;

    fn try_from(value: near_openapi_types::AccountView) -> Result<Self, Self::Error> {
        let near_openapi_types::AccountView {
            amount,
            code_hash,
            global_contract_account_id,
            global_contract_hash,
            locked,
            storage_paid_at: _, // Intentionally ignoring this field. See (https://github.com/near/nearcore/issues/2271)
            storage_usage,
        } = value;

        let code_hash = CryptoHash::try_from(code_hash)?;

        let contract_state = match (code_hash, global_contract_account_id, global_contract_hash) {
            (_, _, Some(hash)) => ContractState::from_global_contract_hash(hash.try_into()?),
            (_, Some(account_id), _) => account_id.into(),
            (hash, _, _) if hash == CryptoHash::default() => ContractState::None,
            (hash, _, _) => ContractState::from_local_hash(hash),
        };

        Ok(Self {
            amount,
            contract_state,
            locked,
            storage_usage,
        })
    }
}
