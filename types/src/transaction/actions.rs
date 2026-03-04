use std::collections::BTreeMap;
use std::str::FromStr;

use crate::AccountId;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::errors::DataConversionError;
use crate::json::U64;
use crate::transaction::delegate_action::SignedDelegateAction;
use crate::utils::near_gas_as_u64;
use crate::{CryptoHash, NearGas, NearToken, PublicKey, Signature};

pub(crate) fn parse_near_token(s: &str) -> Result<NearToken, DataConversionError> {
    Ok(NearToken::from_yoctonear(s.parse::<u128>()?))
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum Action {
    /// Create an (sub)account using a transaction `receiver_id` as an ID for
    /// a new account ID must pass validation rules described here
    /// <http://nomicon.io/Primitives/Account.html>.
    CreateAccount(CreateAccountAction),
    /// Sets a Wasm code to a receiver_id
    DeployContract(DeployContractAction),
    /// Call a function on a contract
    FunctionCall(Box<FunctionCallAction>),
    /// Transfer tokens to an account
    Transfer(TransferAction),
    /// Stake tokens from an account to a validator
    /// As a not a developer of staking pool, you should consider using a staking pool contract instead
    Stake(Box<StakeAction>),
    /// Add a key to an account
    AddKey(Box<AddKeyAction>),
    /// Delete a key from an account
    DeleteKey(Box<DeleteKeyAction>),
    /// Delete an account and transfer all tokens to a beneficiary account
    DeleteAccount(DeleteAccountAction),
    /// Delegate your action submission to some relayer that will cover the cost of the transaction
    Delegate(Box<SignedDelegateAction>),
    /// Deploy a global contract
    DeployGlobalContract(DeployGlobalContractAction),
    /// Use a global contract to link code to an account
    UseGlobalContract(Box<UseGlobalContractAction>),
    /// Deploy a deterministic account with global contract and state.
    ///
    /// See [NEP-616](https://github.com/near/NEPs/blob/master/neps/nep-0616.md) for more details
    DeterministicStateInit(Box<DeterministicStateInitAction>),

    /// Creates a gas key for an account to be used for gas payments
    ///
    /// See [NEP-611](https://github.com/near/NEPs/blob/master/neps/nep-0611.md) for more details
    AddGasKey(Box<AddGasKeyAction>),
    /// Deletes a gas key for an account
    ///
    /// See [NEP-611](https://github.com/near/NEPs/blob/master/neps/nep-0611.md) for more details
    DeleteGasKey(Box<DeleteGasKeyAction>),
    /// Transfers tokens to a gas key
    ///
    /// See [NEP-611](https://github.com/near/NEPs/blob/master/neps/nep-0611.md) for more details
    TransferToGasKey(Box<TransferToGasKeyAction>),
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeterministicStateInitAction {
    pub state_init: DeterministicAccountStateInit,
    pub deposit: NearToken,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum DeterministicAccountStateInit {
    V1(DeterministicAccountStateInitV1),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeterministicAccountStateInitV1 {
    pub code: GlobalContractIdentifier,
    #[serde_as(as = "BTreeMap<Base64, Base64>")]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl TryFrom<near_openrpc_client::DeterministicStateInitAction> for DeterministicStateInitAction {
    type Error = DataConversionError;
    fn try_from(
        val: near_openrpc_client::DeterministicStateInitAction,
    ) -> Result<Self, Self::Error> {
        let near_openrpc_client::DeterministicStateInitAction {
            state_init,
            deposit,
        } = val;

        let deposit = parse_near_token(&deposit)?;

        match state_init {
            near_openrpc_client::DeterministicAccountStateInit::V1(v1) => Ok(Self {
                state_init: DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                    code: v1.code.try_into()?,
                    data: v1
                        .data
                        .into_iter()
                        .map(|(k, v)| {
                            Ok::<(Vec<u8>, Vec<u8>), DataConversionError>((
                                BASE64_STANDARD.decode(k)?,
                                BASE64_STANDARD.decode(v)?,
                            ))
                        })
                        .collect::<Result<BTreeMap<Vec<u8>, Vec<u8>>, _>>()?,
                }),
                deposit,
            }),
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployGlobalContractAction {
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
    pub deploy_mode: GlobalContractDeployMode,
}

impl TryFrom<near_openrpc_client::DeployGlobalContractAction> for DeployGlobalContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::DeployGlobalContractAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::DeployGlobalContractAction { code, deploy_mode } = val;
        Ok(Self {
            code: BASE64_STANDARD.decode(code)?,
            deploy_mode: deploy_mode.into(),
        })
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct UseGlobalContractAction {
    pub contract_identifier: GlobalContractIdentifier,
}

impl TryFrom<near_openrpc_client::UseGlobalContractAction> for UseGlobalContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::UseGlobalContractAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::UseGlobalContractAction {
            contract_identifier,
        } = val;
        Ok(Self {
            contract_identifier: contract_identifier.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct CreateAccountAction {}

impl From<near_openrpc_client::CreateAccountAction> for CreateAccountAction {
    fn from(_val: near_openrpc_client::CreateAccountAction) -> Self {
        Self {}
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployContractAction {
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
}

impl TryFrom<near_openrpc_client::DeployContractAction> for DeployContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::DeployContractAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::DeployContractAction { code } = val;
        Ok(Self {
            code: BASE64_STANDARD.decode(code)?,
        })
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallAction {
    pub method_name: String,
    #[serde_as(as = "Base64")]
    pub args: Vec<u8>,
    #[serde(serialize_with = "near_gas_as_u64::serialize")]
    pub gas: NearGas,
    pub deposit: NearToken,
}

impl TryFrom<near_openrpc_client::FunctionCallAction> for FunctionCallAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::FunctionCallAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::FunctionCallAction {
            method_name,
            args,
            gas,
            deposit,
        } = val;
        Ok(Self {
            method_name,
            args: BASE64_STANDARD.decode(&*args)?,
            gas: NearGas::from_gas(*gas),
            deposit: parse_near_token(&deposit)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct TransferAction {
    pub deposit: NearToken,
}

impl TryFrom<near_openrpc_client::TransferAction> for TransferAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::TransferAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::TransferAction { deposit } = val;
        Ok(Self {
            deposit: parse_near_token(&deposit)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct StakeAction {
    /// Amount of tokens to stake.
    pub stake: NearToken,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

impl TryFrom<near_openrpc_client::StakeAction> for StakeAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::StakeAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::StakeAction { public_key, stake } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
            stake: parse_near_token(&stake)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AddGasKeyAction {
    pub public_key: PublicKey,
    pub num_nonces: u32,
    pub permission: AccessKeyPermission,
}


#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteGasKeyAction {
    pub public_key: PublicKey,
}


#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct TransferToGasKeyAction {
    pub public_key: PublicKey,
    pub deposit: NearToken,
}

impl TryFrom<near_openrpc_client::TransferToGasKeyAction> for TransferToGasKeyAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::TransferToGasKeyAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::TransferToGasKeyAction {
            public_key,
            deposit,
        } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
            deposit: parse_near_token(&deposit)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AddKeyAction {
    /// A public key which will be associated with an access_key
    pub public_key: PublicKey,
    /// An access key with the permission
    pub access_key: AccessKey,
}

impl TryFrom<near_openrpc_client::AddKeyAction> for AddKeyAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::AddKeyAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::AddKeyAction {
            public_key,
            access_key,
        } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
            access_key: access_key.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AccessKey {
    /// Nonce for this access key, used for tx nonce generation. When access key is created, nonce
    /// is set to `(block_height - 1) * 1e6` to avoid tx hash collision on access key re-creation.
    /// See <https://github.com/near/nearcore/issues/3779> for more details.
    pub nonce: U64,
    /// Defines permissions for this access key.
    pub permission: AccessKeyPermission,
}

impl TryFrom<near_openrpc_client::AccessKeyView> for AccessKey {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::AccessKeyView) -> Result<Self, Self::Error> {
        let near_openrpc_client::AccessKeyView { nonce, permission } = val;
        Ok(Self {
            nonce: U64(nonce),
            permission: permission.try_into()?,
        })
    }
}

impl TryFrom<near_openrpc_client::AccessKey> for AccessKey {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::AccessKey) -> Result<Self, Self::Error> {
        let near_openrpc_client::AccessKey { nonce, permission } = val;
        Ok(Self {
            nonce: U64(nonce),
            permission: permission.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum AccessKeyPermission {
    FunctionCall(FunctionCallPermission),
    /// Grants full access to the account.
    /// NOTE: It's used to replace account-level public keys.
    FullAccess,
}

impl TryFrom<near_openrpc_client::AccessKeyPermissionView> for AccessKeyPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::AccessKeyPermissionView) -> Result<Self, Self::Error> {
        match val {
            near_openrpc_client::AccessKeyPermissionView::FunctionCall {
                allowance,
                method_names,
                receiver_id,
            } => Ok(Self::FunctionCall(FunctionCallPermission {
                allowance: allowance.map(|a| parse_near_token(&a)).transpose()?,
                receiver_id,
                method_names,
            })),
            near_openrpc_client::AccessKeyPermissionView::FullAccess => Ok(Self::FullAccess),
            near_openrpc_client::AccessKeyPermissionView::GasKeyFunctionCall { .. }
            | near_openrpc_client::AccessKeyPermissionView::GasKeyFullAccess { .. } => {
                Ok(Self::FullAccess)
            }
        }
    }
}

impl TryFrom<near_openrpc_client::AccessKeyPermission> for AccessKeyPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::AccessKeyPermission) -> Result<Self, Self::Error> {
        match val {
            near_openrpc_client::AccessKeyPermission::FunctionCall(function_call_permission) => {
                Ok(Self::FunctionCall(function_call_permission.try_into()?))
            }
            near_openrpc_client::AccessKeyPermission::FullAccess => Ok(Self::FullAccess),
            near_openrpc_client::AccessKeyPermission::GasKeyFunctionCall(_, function_call_permission) => {
                Ok(Self::FunctionCall(function_call_permission.try_into()?))
            }
            near_openrpc_client::AccessKeyPermission::GasKeyFullAccess(_) => Ok(Self::FullAccess),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallPermission {
    pub allowance: Option<NearToken>,
    pub receiver_id: String,
    pub method_names: Vec<String>,
}

impl TryFrom<near_openrpc_client::FunctionCallPermission> for FunctionCallPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::FunctionCallPermission) -> Result<Self, Self::Error> {
        let near_openrpc_client::FunctionCallPermission {
            allowance,
            receiver_id,
            method_names,
        } = val;
        Ok(Self {
            allowance: allowance.map(|a| parse_near_token(&a)).transpose()?,
            receiver_id,
            method_names,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteKeyAction {
    /// A public key associated with the access_key to be deleted.
    pub public_key: PublicKey,
}

impl TryFrom<near_openrpc_client::DeleteKeyAction> for DeleteKeyAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::DeleteKeyAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::DeleteKeyAction { public_key } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
}

impl TryFrom<near_openrpc_client::DeleteAccountAction> for DeleteAccountAction {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::DeleteAccountAction) -> Result<Self, Self::Error> {
        let near_openrpc_client::DeleteAccountAction { beneficiary_id } = val;
        Ok(Self {
            beneficiary_id: beneficiary_id.parse()?,
        })
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
)]
#[repr(u8)]
pub enum GlobalContractDeployMode {
    /// Contract is deployed under its code hash.
    /// Users will be able reference it by that hash.
    /// This effectively makes the contract immutable.
    CodeHash,
    /// Contract is deployed under the owner account id.
    /// Users will be able reference it by that account id.
    /// This allows the owner to update the contract for all its users.
    AccountId,
}

impl From<near_openrpc_client::GlobalContractDeployMode> for GlobalContractDeployMode {
    fn from(val: near_openrpc_client::GlobalContractDeployMode) -> Self {
        match val {
            near_openrpc_client::GlobalContractDeployMode::CodeHash => Self::CodeHash,
            near_openrpc_client::GlobalContractDeployMode::AccountId => Self::AccountId,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GlobalContractIdentifier {
    CodeHash(CryptoHash),
    AccountId(AccountId),
}

impl TryFrom<near_openrpc_client::GlobalContractIdentifier> for GlobalContractIdentifier {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::GlobalContractIdentifier) -> Result<Self, Self::Error> {
        match val {
            near_openrpc_client::GlobalContractIdentifier::CodeHash(code_hash) => {
                Ok(Self::CodeHash(code_hash.try_into()?))
            }
            near_openrpc_client::GlobalContractIdentifier::AccountId(account_id) => {
                Ok(Self::AccountId(account_id.parse()?))
            }
        }
    }
}

impl TryFrom<near_openrpc_client::GlobalContractIdentifierView> for GlobalContractIdentifier {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::GlobalContractIdentifierView) -> Result<Self, Self::Error> {
        match val {
            near_openrpc_client::GlobalContractIdentifierView::Hash(code_hash) => {
                Ok(Self::CodeHash(code_hash.try_into()?))
            }
            near_openrpc_client::GlobalContractIdentifierView::AccountId(account_id) => {
                Ok(Self::AccountId(account_id.parse()?))
            }
        }
    }
}

impl TryFrom<near_openrpc_client::ActionView> for Action {
    type Error = DataConversionError;
    fn try_from(val: near_openrpc_client::ActionView) -> Result<Self, Self::Error> {
        match val {
            near_openrpc_client::ActionView::DeterministicStateInit {
                code,
                data,
                deposit,
            } => Ok(Self::DeterministicStateInit(Box::new(
                DeterministicStateInitAction {
                    state_init: DeterministicAccountStateInit::V1(
                        DeterministicAccountStateInitV1 {
                            code: code.try_into()?,
                            data: data
                                .into_iter()
                                .map(|(k, v)| {
                                    Ok::<(Vec<u8>, Vec<u8>), DataConversionError>((
                                        BASE64_STANDARD.decode(k)?,
                                        BASE64_STANDARD.decode(v)?,
                                    ))
                                })
                                .collect::<Result<BTreeMap<Vec<u8>, Vec<u8>>, _>>()?,
                        },
                    ),
                    deposit: parse_near_token(&deposit)?,
                },
            ))),
            near_openrpc_client::ActionView::CreateAccount => {
                Ok(Self::CreateAccount(CreateAccountAction {}))
            }
            near_openrpc_client::ActionView::DeployContract { code } => {
                Ok(Self::DeployContract(DeployContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                }))
            }
            near_openrpc_client::ActionView::FunctionCall {
                method_name,
                args,
                gas,
                deposit,
            } => Ok(Self::FunctionCall(Box::new(FunctionCallAction {
                method_name,
                args: BASE64_STANDARD.decode(&*args)?,
                gas: NearGas::from_gas(*gas),
                deposit: parse_near_token(&deposit)?,
            }))),
            near_openrpc_client::ActionView::Transfer { deposit } => {
                Ok(Self::Transfer(TransferAction {
                    deposit: parse_near_token(&deposit)?,
                }))
            }
            near_openrpc_client::ActionView::Stake { public_key, stake } => {
                Ok(Self::Stake(Box::new(StakeAction {
                    public_key: public_key.try_into()?,
                    stake: parse_near_token(&stake)?,
                })))
            }
            near_openrpc_client::ActionView::AddKey {
                access_key,
                public_key,
            } => Ok(Self::AddKey(Box::new(AddKeyAction {
                public_key: public_key.try_into()?,
                access_key: access_key.try_into()?,
            }))),
            near_openrpc_client::ActionView::DeleteKey { public_key } => {
                Ok(Self::DeleteKey(Box::new(DeleteKeyAction {
                    public_key: public_key.try_into()?,
                })))
            }
            near_openrpc_client::ActionView::DeleteAccount { beneficiary_id } => {
                Ok(Self::DeleteAccount(DeleteAccountAction {
                    beneficiary_id: beneficiary_id.parse()?,
                }))
            }
            near_openrpc_client::ActionView::Delegate {
                delegate_action,
                signature,
            } => Ok(Self::Delegate(Box::new(SignedDelegateAction {
                delegate_action: delegate_action.try_into()?,
                signature: Signature::from_str(&signature)?,
            }))),
            near_openrpc_client::ActionView::DeployGlobalContract { code } => {
                Ok(Self::DeployGlobalContract(DeployGlobalContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                    deploy_mode: GlobalContractDeployMode::CodeHash,
                }))
            }
            near_openrpc_client::ActionView::DeployGlobalContractByAccountId { code } => {
                Ok(Self::DeployGlobalContract(DeployGlobalContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                    deploy_mode: GlobalContractDeployMode::AccountId,
                }))
            }
            near_openrpc_client::ActionView::UseGlobalContract { code_hash } => {
                Ok(Self::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::CodeHash(code_hash.try_into()?),
                })))
            }
            near_openrpc_client::ActionView::UseGlobalContractByAccountId { account_id } => {
                Ok(Self::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::AccountId(account_id.parse()?),
                })))
            }
            near_openrpc_client::ActionView::TransferToGasKey {
                deposit,
                public_key,
            } => Ok(Self::TransferToGasKey(Box::new(TransferToGasKeyAction {
                public_key: public_key.try_into()?,
                deposit: parse_near_token(&deposit)?,
            }))),
            near_openrpc_client::ActionView::WithdrawFromGasKey {
                amount,
                public_key,
            } => Ok(Self::TransferToGasKey(Box::new(TransferToGasKeyAction {
                public_key: public_key.try_into()?,
                deposit: parse_near_token(&amount)?,
            }))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::*;
    use crate::crypto::{ED25519_PUBLIC_KEY_LENGTH, public_key::ED25519PublicKey};
    use crate::transaction::delegate_action::{DelegateAction, NonDelegateAction};
    use near_primitives::action as npa;
    use near_primitives::deterministic_account_id::{
        DeterministicAccountStateInit as npaDeterministicAccountStateInit,
        DeterministicAccountStateInitV1 as npaDeterministicAccountStateInitV1,
    };
    use near_primitives::gas::Gas;
    use near_primitives::global_contract::GlobalContractIdentifier as npaGlobalContractIdentifier;
    use serde_json;

    fn get_actions() -> (Vec<Action>, Vec<npa::Action>) {
        let btreemap = BTreeMap::from([(b"key".to_vec(), b"value".to_vec())]);

        let local_actions = vec![
            Action::CreateAccount(CreateAccountAction {}),
            Action::DeployContract(DeployContractAction {
                code: vec![1, 2, 3],
            }),
            Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: "test".to_string(),
                args: vec![4, 5, 6],
                gas: NearGas::from_gas(1000000),
                deposit: NearToken::from_yoctonear(0),
            })),
            Action::Transfer(TransferAction {
                deposit: NearToken::from_yoctonear(1000000000),
            }),
            Action::Stake(Box::new(StakeAction {
                stake: NearToken::from_yoctonear(100000000),
                public_key: PublicKey::ED25519(ED25519PublicKey([0; ED25519_PUBLIC_KEY_LENGTH])),
            })),
            Action::AddKey(Box::new(AddKeyAction {
                public_key: PublicKey::ED25519(ED25519PublicKey([0; ED25519_PUBLIC_KEY_LENGTH])),
                access_key: AccessKey {
                    nonce: U64(0),
                    permission: AccessKeyPermission::FullAccess,
                },
            })),
            Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key: PublicKey::ED25519(ED25519PublicKey([0; ED25519_PUBLIC_KEY_LENGTH])),
            })),
            Action::DeleteAccount(DeleteAccountAction {
                beneficiary_id: "alice.near".parse().unwrap(),
            }),
            Action::DeployGlobalContract(DeployGlobalContractAction {
                code: vec![7, 8, 9],
                deploy_mode: GlobalContractDeployMode::CodeHash,
            }),
            Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                contract_identifier: GlobalContractIdentifier::AccountId(
                    "global.near".parse().unwrap(),
                ),
            })),
            Action::Delegate(Box::new(SignedDelegateAction {
                delegate_action: DelegateAction {
                    sender_id: "sender.near".parse().unwrap(),
                    receiver_id: "receiver.near".parse().unwrap(),
                    actions: vec![
                        NonDelegateAction::try_from(Action::Transfer(TransferAction {
                            deposit: NearToken::from_yoctonear(1000),
                        }))
                        .unwrap(),
                    ],
                    nonce: 1,
                    max_block_height: 1000,
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        [0; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                },
                signature: Signature::from_parts(crate::crypto::KeyType::ED25519, &[0u8; 64])
                    .unwrap(),
            })),
            Action::DeterministicStateInit(Box::new(DeterministicStateInitAction {
                state_init: DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
                    code: GlobalContractIdentifier::AccountId("init.near".parse().unwrap()),
                    data: btreemap.clone(),
                }),
                deposit: NearToken::from_yoctonear(5000000000),
            })),
        ];

        let near_primitives_actions = vec![
            npa::Action::CreateAccount(npa::CreateAccountAction {}),
            npa::Action::DeployContract(npa::DeployContractAction {
                code: vec![1, 2, 3],
            }),
            npa::Action::FunctionCall(Box::new(npa::FunctionCallAction {
                method_name: "test".to_string(),
                args: vec![4, 5, 6],
                gas: Gas::from_gas(1000000),
                deposit: NearToken::ZERO,
            })),
            npa::Action::Transfer(npa::TransferAction {
                deposit: NearToken::from_yoctonear(1000000000),
            }),
            npa::Action::Stake(Box::new(npa::StakeAction {
                stake: NearToken::from_yoctonear(100000000),
                public_key: near_crypto::PublicKey::empty(near_crypto::KeyType::ED25519),
            })),
            npa::Action::AddKey(Box::new(npa::AddKeyAction {
                public_key: near_crypto::PublicKey::empty(near_crypto::KeyType::ED25519),
                access_key: near_primitives::account::AccessKey {
                    nonce: 0,
                    permission: near_primitives::account::AccessKeyPermission::FullAccess,
                },
            })),
            npa::Action::DeleteKey(Box::new(npa::DeleteKeyAction {
                public_key: near_crypto::PublicKey::empty(near_crypto::KeyType::ED25519),
            })),
            npa::Action::DeleteAccount(npa::DeleteAccountAction {
                beneficiary_id: "alice.near".parse().unwrap(),
            }),
            npa::Action::DeployGlobalContract(npa::DeployGlobalContractAction {
                code: Arc::new([7, 8, 9]),
                deploy_mode: npa::GlobalContractDeployMode::CodeHash,
            }),
            npa::Action::UseGlobalContract(Box::new(npa::UseGlobalContractAction {
                contract_identifier: npa::GlobalContractIdentifier::AccountId(
                    "global.near".parse().unwrap(),
                ),
            })),
            npa::Action::Delegate(Box::new(npa::delegate::SignedDelegateAction {
                delegate_action: npa::delegate::DelegateAction {
                    sender_id: "sender.near".parse().unwrap(),
                    receiver_id: "receiver.near".parse().unwrap(),
                    actions: vec![
                        npa::delegate::NonDelegateAction::try_from(npa::Action::Transfer(
                            npa::TransferAction {
                                deposit: NearToken::from_yoctonear(1000),
                            },
                        ))
                        .unwrap(),
                    ],
                    nonce: 1,
                    max_block_height: 1000,
                    public_key: near_crypto::PublicKey::empty(near_crypto::KeyType::ED25519),
                },
                signature: near_crypto::Signature::from_parts(
                    near_crypto::KeyType::ED25519,
                    &[0u8; 64],
                )
                .unwrap(),
            })),
            npa::Action::DeterministicStateInit(Box::new(npa::DeterministicStateInitAction {
                state_init: npaDeterministicAccountStateInit::V1(
                    npaDeterministicAccountStateInitV1 {
                        code: npaGlobalContractIdentifier::AccountId("init.near".parse().unwrap()),
                        data: btreemap,
                    },
                ),
                deposit: NearToken::from_yoctonear(5000000000),
            })),
        ];

        (local_actions, near_primitives_actions)
    }

    #[test]
    fn test_action_serialization() {
        let (local_actions, _) = get_actions();

        for action in local_actions {
            let serialized =
                serde_json::to_string(&action).expect("Failed to serialize action to JSON");

            let deserialized: Action =
                serde_json::from_str(&serialized).expect("Failed to deserialize action from JSON");

            assert_eq!(
                action, deserialized,
                "Serialization/Deserialization mismatch: original action: {action:?}, deserialized action: {deserialized:?}"
            );
        }
    }

    #[test]
    fn test_action_borsh_serialization() {
        let (local_actions, _) = get_actions();

        for action in local_actions {
            let serialized = borsh::to_vec(&action).expect("Failed to serialize action to borsh");

            let deserialized: Action = Action::try_from_slice(&serialized)
                .expect("Failed to deserialize action from borsh");

            assert_eq!(
                action, deserialized,
                "Serialization/Deserialization mismatch: original action: {action:?}, deserialized action: {deserialized:?}"
            );
        }
    }

    #[test]
    fn serialization_comparison_with_near_primitives() {
        let (local_actions, near_primitives_actions) = get_actions();

        assert_eq!(
            local_actions.len(),
            near_primitives_actions.len(),
            "Action lists should have the same length"
        );

        for (local_action, np_action) in local_actions.iter().zip(near_primitives_actions.iter()) {
            // Compare borsh serialization - this is what matters for on-chain compatibility
            let local_borsh =
                borsh::to_vec(local_action).expect("Failed to serialize local action to borsh");
            let np_borsh = borsh::to_vec(np_action)
                .expect("Failed to serialize near_primitives action to borsh");

            assert_eq!(local_borsh, np_borsh, "Borsh serialization mismatch");

            // Note: We intentionally diverge from near_primitives JSON serialization format.
            // Our U64 and U128 types serialize as strings for JavaScript compatibility,
            // while near_primitives uses numbers. This is by design - JSON is for API/RPC
            // communication where JavaScript compatibility matters, while Borsh is for
            // on-chain data where binary compatibility is what counts.
        }
    }
}
