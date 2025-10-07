use std::collections::BTreeMap;
use std::str::FromStr;

use crate::AccountId;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::errors::DataConversionError;
use crate::json::U64;
use crate::transaction::delegate_action::SignedDelegateAction;
use crate::{CryptoHash, NearGas, NearToken, PublicKey, Signature};

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
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeterministicStateInitAction {
    pub code: GlobalContractIdentifier,
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
    pub deposit: NearToken,
}

impl TryFrom<near_openapi_types::DeterministicStateInitAction> for DeterministicStateInitAction {
    type Error = DataConversionError;
    fn try_from(
        val: near_openapi_types::DeterministicStateInitAction,
    ) -> Result<Self, Self::Error> {
        let near_openapi_types::DeterministicStateInitAction {
            state_init,
            deposit,
        } = val;

        match state_init {
            near_openapi_types::DeterministicAccountStateInit::V1(v1) => Ok(Self {
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
                deposit,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployGlobalContractAction {
    pub code: Vec<u8>,
    pub deploy_mode: GlobalContractDeployMode,
}

impl TryFrom<near_openapi_types::DeployGlobalContractAction> for DeployGlobalContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::DeployGlobalContractAction) -> Result<Self, Self::Error> {
        let near_openapi_types::DeployGlobalContractAction { code, deploy_mode } = val;
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

impl TryFrom<near_openapi_types::UseGlobalContractAction> for UseGlobalContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::UseGlobalContractAction) -> Result<Self, Self::Error> {
        let near_openapi_types::UseGlobalContractAction {
            contract_identifier,
        } = val;
        Ok(Self {
            contract_identifier: contract_identifier.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct CreateAccountAction {}

impl From<near_openapi_types::CreateAccountAction> for CreateAccountAction {
    fn from(_: near_openapi_types::CreateAccountAction) -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployContractAction {
    pub code: Vec<u8>,
}

impl TryFrom<near_openapi_types::DeployContractAction> for DeployContractAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::DeployContractAction) -> Result<Self, Self::Error> {
        let near_openapi_types::DeployContractAction { code } = val;
        Ok(Self {
            code: BASE64_STANDARD.decode(code)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallAction {
    pub method_name: String,
    pub args: Vec<u8>,
    pub gas: NearGas,
    pub deposit: NearToken,
}

impl TryFrom<near_openapi_types::FunctionCallAction> for FunctionCallAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::FunctionCallAction) -> Result<Self, Self::Error> {
        let near_openapi_types::FunctionCallAction {
            method_name,
            args,
            gas,
            deposit,
        } = val;
        Ok(Self {
            method_name,
            args: BASE64_STANDARD.decode(args)?,
            gas,
            deposit,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct TransferAction {
    pub deposit: NearToken,
}

impl TryFrom<near_openapi_types::TransferAction> for TransferAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::TransferAction) -> Result<Self, Self::Error> {
        let near_openapi_types::TransferAction { deposit } = val;
        Ok(Self { deposit })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct StakeAction {
    /// Amount of tokens to stake.
    pub stake: NearToken,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

impl TryFrom<near_openapi_types::StakeAction> for StakeAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::StakeAction) -> Result<Self, Self::Error> {
        let near_openapi_types::StakeAction { public_key, stake } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
            stake,
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

impl TryFrom<near_openapi_types::AddKeyAction> for AddKeyAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::AddKeyAction) -> Result<Self, Self::Error> {
        let near_openapi_types::AddKeyAction {
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

impl TryFrom<near_openapi_types::AccessKeyView> for AccessKey {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::AccessKeyView) -> Result<Self, Self::Error> {
        let near_openapi_types::AccessKeyView { nonce, permission } = val;
        Ok(Self {
            nonce: U64(nonce),
            permission: permission.try_into()?,
        })
    }
}

impl TryFrom<near_openapi_types::AccessKey> for AccessKey {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::AccessKey) -> Result<Self, Self::Error> {
        let near_openapi_types::AccessKey { nonce, permission } = val;
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

impl TryFrom<near_openapi_types::AccessKeyPermissionView> for AccessKeyPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::AccessKeyPermissionView) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::AccessKeyPermissionView::FunctionCall {
                allowance,
                method_names,
                receiver_id,
            } => Ok(Self::FunctionCall(FunctionCallPermission {
                allowance,
                receiver_id,
                method_names,
            })),
            near_openapi_types::AccessKeyPermissionView::FullAccess => Ok(Self::FullAccess),
        }
    }
}

impl TryFrom<near_openapi_types::AccessKeyPermission> for AccessKeyPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::AccessKeyPermission) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::AccessKeyPermission::FunctionCall(function_call_permission) => {
                Ok(Self::FunctionCall(function_call_permission.try_into()?))
            }
            near_openapi_types::AccessKeyPermission::FullAccess => Ok(Self::FullAccess),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallPermission {
    pub allowance: Option<NearToken>,
    pub receiver_id: String,
    pub method_names: Vec<String>,
}

impl TryFrom<near_openapi_types::FunctionCallPermission> for FunctionCallPermission {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::FunctionCallPermission) -> Result<Self, Self::Error> {
        let near_openapi_types::FunctionCallPermission {
            allowance,
            receiver_id,
            method_names,
        } = val;
        Ok(Self {
            allowance,
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

impl TryFrom<near_openapi_types::DeleteKeyAction> for DeleteKeyAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::DeleteKeyAction) -> Result<Self, Self::Error> {
        let near_openapi_types::DeleteKeyAction { public_key } = val;
        Ok(Self {
            public_key: public_key.try_into()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
}

impl From<near_openapi_types::DeleteAccountAction> for DeleteAccountAction {
    fn from(val: near_openapi_types::DeleteAccountAction) -> Self {
        let near_openapi_types::DeleteAccountAction { beneficiary_id } = val;
        Self { beneficiary_id }
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

impl From<near_openapi_types::GlobalContractDeployMode> for GlobalContractDeployMode {
    fn from(val: near_openapi_types::GlobalContractDeployMode) -> Self {
        match val {
            near_openapi_types::GlobalContractDeployMode::CodeHash => Self::CodeHash,
            near_openapi_types::GlobalContractDeployMode::AccountId => Self::AccountId,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GlobalContractIdentifier {
    CodeHash(CryptoHash),
    AccountId(AccountId),
}

impl TryFrom<near_openapi_types::GlobalContractIdentifier> for GlobalContractIdentifier {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::GlobalContractIdentifier) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::GlobalContractIdentifier::CodeHash(code_hash) => {
                Ok(Self::CodeHash(code_hash.try_into()?))
            }
            near_openapi_types::GlobalContractIdentifier::AccountId(account_id) => {
                Ok(Self::AccountId(account_id))
            }
        }
    }
}

impl TryFrom<near_openapi_types::GlobalContractIdentifierView> for GlobalContractIdentifier {
    type Error = DataConversionError;
    fn try_from(
        val: near_openapi_types::GlobalContractIdentifierView,
    ) -> Result<Self, Self::Error> {
        let near_openapi_types::GlobalContractIdentifierView {
            subtype_0: code_hash,
            subtype_1: account_id,
        } = val;
        if let Some(code_hash) = code_hash {
            Ok(Self::CodeHash(code_hash.try_into()?))
        } else if let Some(account_id) = account_id {
            Ok(Self::AccountId(account_id))
        } else {
            Err(DataConversionError::InvalidGlobalContractIdentifier)
        }
    }
}

impl TryFrom<near_openapi_types::ActionView> for Action {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::ActionView) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::ActionView::DeterministicStateInit {
                code,
                data,
                deposit,
            } => Ok(Self::DeterministicStateInit(Box::new(
                DeterministicStateInitAction {
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
                    deposit,
                },
            ))),
            near_openapi_types::ActionView::CreateAccount => {
                Ok(Self::CreateAccount(CreateAccountAction {}))
            }
            near_openapi_types::ActionView::DeployContract { code } => {
                Ok(Self::DeployContract(DeployContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                }))
            }
            near_openapi_types::ActionView::FunctionCall {
                method_name,
                args,
                gas,
                deposit,
            } => Ok(Self::FunctionCall(Box::new(FunctionCallAction {
                method_name,
                args: BASE64_STANDARD.decode(args.0)?,
                gas,
                deposit,
            }))),
            near_openapi_types::ActionView::Transfer { deposit } => {
                Ok(Self::Transfer(TransferAction { deposit }))
            }
            near_openapi_types::ActionView::Stake { public_key, stake } => {
                Ok(Self::Stake(Box::new(StakeAction {
                    public_key: public_key.try_into()?,
                    stake,
                })))
            }
            near_openapi_types::ActionView::AddKey {
                access_key,
                public_key,
            } => Ok(Self::AddKey(Box::new(AddKeyAction {
                public_key: public_key.try_into()?,
                access_key: access_key.try_into()?,
            }))),
            near_openapi_types::ActionView::DeleteKey { public_key } => {
                Ok(Self::DeleteKey(Box::new(DeleteKeyAction {
                    public_key: public_key.try_into()?,
                })))
            }
            near_openapi_types::ActionView::DeleteAccount { beneficiary_id } => {
                Ok(Self::DeleteAccount(DeleteAccountAction { beneficiary_id }))
            }
            near_openapi_types::ActionView::Delegate {
                delegate_action,
                signature,
            } => Ok(Self::Delegate(Box::new(SignedDelegateAction {
                delegate_action: delegate_action.try_into()?,
                signature: Signature::from_str(&signature)?,
            }))),
            near_openapi_types::ActionView::DeployGlobalContract { code } => {
                Ok(Self::DeployGlobalContract(DeployGlobalContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                    deploy_mode: GlobalContractDeployMode::CodeHash,
                }))
            }
            near_openapi_types::ActionView::DeployGlobalContractByAccountId { code } => {
                Ok(Self::DeployGlobalContract(DeployGlobalContractAction {
                    code: BASE64_STANDARD.decode(code)?,
                    deploy_mode: GlobalContractDeployMode::AccountId,
                }))
            }
            near_openapi_types::ActionView::UseGlobalContract { code_hash } => {
                Ok(Self::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::CodeHash(code_hash.try_into()?),
                })))
            }
            near_openapi_types::ActionView::UseGlobalContractByAccountId { account_id } => {
                Ok(Self::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::AccountId(account_id),
                })))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{ED25519_PUBLIC_KEY_LENGTH, public_key::ED25519PublicKey};
    use serde_json;

    fn get_actions() -> Vec<Action> {
        vec![
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
                public_key: PublicKey::ED25519(ED25519PublicKey([1; ED25519_PUBLIC_KEY_LENGTH])),
                access_key: AccessKey {
                    nonce: U64(0),
                    permission: AccessKeyPermission::FullAccess,
                },
            })),
            Action::DeleteKey(Box::new(DeleteKeyAction {
                public_key: PublicKey::ED25519(ED25519PublicKey([2; ED25519_PUBLIC_KEY_LENGTH])),
            })),
            Action::DeleteAccount(DeleteAccountAction {
                beneficiary_id: "alice.near".parse().unwrap(),
            }),
        ]
    }

    #[test]
    fn test_action_serialization() {
        let actions = get_actions();

        for action in actions {
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
        let actions = get_actions();

        for action in actions {
            let serialized = borsh::to_vec(&action).expect("Failed to serialize action to borsh");

            let deserialized: Action = Action::try_from_slice(&serialized)
                .expect("Failed to deserialize action from borsh");

            assert_eq!(
                action, deserialized,
                "Serialization/Deserialization mismatch: original action: {action:?}, deserialized action: {deserialized:?}"
            );
        }
    }
}
