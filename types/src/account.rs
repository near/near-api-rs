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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub amount: NearToken,
    pub contract_state: ContractState,
    pub locked: NearToken,
    pub storage_usage: StorageUsage,
}

impl TryFrom<near_openrpc_client::RpcViewAccountResponse> for Account {
    type Error = DataConversionError;

    fn try_from(value: near_openrpc_client::RpcViewAccountResponse) -> Result<Self, Self::Error> {
        let near_openrpc_client::RpcViewAccountResponse {
            amount,
            block_hash: _,
            block_height: _,
            code_hash,
            global_contract_account_id,
            global_contract_hash,
            locked,
            storage_paid_at: _, // Intentionally ignoring this field. See (https://github.com/near/nearcore/issues/2271)
            storage_usage,
        } = value;

        let amount = NearToken::from_yoctonear(amount.parse::<u128>()?);
        let locked = NearToken::from_yoctonear(locked.parse::<u128>()?);
        let code_hash = CryptoHash::try_from(code_hash)?;

        let global_contract_hash = global_contract_hash.map(CryptoHash::try_from).transpose()?;
        let global_contract_account_id: Option<AccountId> = global_contract_account_id
            .map(|id| id.parse::<AccountId>())
            .transpose()?;

        let contract_state = match (code_hash, global_contract_account_id, global_contract_hash) {
            (_, _, Some(hash)) => ContractState::from_global_contract_hash(hash),
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

impl serde::Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let version = AccountVersion::V2;
        let code_hash = match self.contract_state {
            ContractState::LocalHash(hash) => hash,
            _ => CryptoHash::default(),
        };
        let repr = SerdeAccount {
            amount: self.amount,
            locked: self.locked,
            code_hash,
            storage_usage: self.storage_usage,
            version,
            global_contract_hash: match &self.contract_state {
                ContractState::GlobalHash(hash) => Some(*hash),
                _ => None,
            },
            global_contract_account_id: match &self.contract_state {
                ContractState::GlobalAccountId(account_id) => Some(account_id.clone()),
                _ => None,
            },
        };
        serde::Serialize::serialize(&repr, serializer)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum AccountVersion {
    V1,
    V2,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct SerdeAccount {
    pub amount: NearToken,
    pub locked: NearToken,
    pub code_hash: CryptoHash,
    pub storage_usage: u64,
    pub version: AccountVersion,
    pub global_contract_hash: Option<CryptoHash>,
    pub global_contract_account_id: Option<AccountId>,
}
