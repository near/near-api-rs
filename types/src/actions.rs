use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::AccountId;
use near_sdk::serde::{Deserialize, Serialize};

use crate::delegate_action::SignedDelegateAction;
use crate::errors::AccessKeyError;
use crate::integers::{U64, U128};
use crate::{CryptoHash, PublicKey};

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum Action {
    /// Create an (sub)account using a transaction `receiver_id` as an ID for
    /// a new account ID must pass validation rules described here
    /// <http://nomicon.io/Primitives/Account.html>.
    CreateAccount(CreateAccountAction),
    /// Sets a Wasm code to a receiver_id
    DeployContract(DeployContractAction),
    FunctionCall(Box<FunctionCallAction>),
    Transfer(TransferAction),
    Stake(Box<StakeAction>),
    AddKey(Box<AddKeyAction>),
    DeleteKey(Box<DeleteKeyAction>),
    DeleteAccount(DeleteAccountAction),
    Delegate(Box<SignedDelegateAction>),
    DeployGlobalContract(DeployGlobalContractAction),
    UseGlobalContract(Box<UseGlobalContractAction>),
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployGlobalContractAction {
    pub code: Vec<u8>,
    pub deploy_mode: GlobalContractDeployMode,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct UseGlobalContractAction {
    pub contract_identifier: GlobalContractIdentifier,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct CreateAccountAction {}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeployContractAction {
    pub code: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallAction {
    pub method_name: String,
    pub args: Vec<u8>,
    pub gas: U64,
    pub deposit: U128,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct TransferAction {
    pub deposit: U128,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct StakeAction {
    /// Amount of tokens to stake.
    pub stake: U128,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AddKeyAction {
    /// A public key which will be associated with an access_key
    pub public_key: PublicKey,
    /// An access key with the permission
    pub access_key: AccessKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct AccessKeyInfo {
    pub public_key: PublicKey,
    pub access_key: AccessKey,
}

impl TryFrom<near_openapi_types::AccessKeyInfoView> for AccessKeyInfo {
    type Error = AccessKeyError;
    fn try_from(val: near_openapi_types::AccessKeyInfoView) -> Result<Self, Self::Error> {
        let near_openapi_types::AccessKeyInfoView {
            public_key,
            access_key,
        } = val;

        Ok(AccessKeyInfo {
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
    type Error = std::num::ParseIntError;
    fn try_from(val: near_openapi_types::AccessKeyView) -> Result<Self, Self::Error> {
        let near_openapi_types::AccessKeyView { nonce, permission } = val;
        Ok(AccessKey {
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
    type Error = std::num::ParseIntError;
    fn try_from(val: near_openapi_types::AccessKeyPermissionView) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::AccessKeyPermissionView::FunctionCall {
                allowance,
                method_names,
                receiver_id,
            } => {
                let allowance = if let Some(val) = allowance {
                    Some(U128(val.parse::<u128>()?))
                } else {
                    None
                };
                Ok(AccessKeyPermission::FunctionCall(FunctionCallPermission {
                    allowance,
                    receiver_id,
                    method_names,
                }))
            }
            near_openapi_types::AccessKeyPermissionView::FullAccess => {
                Ok(AccessKeyPermission::FullAccess)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct FunctionCallPermission {
    pub allowance: Option<U128>,
    pub receiver_id: String,
    pub method_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteKeyAction {
    /// A public key associated with the access_key to be deleted.
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
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
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum GlobalContractIdentifier {
    CodeHash(CryptoHash),
    AccountId(AccountId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_key::{ED25519_PUBLIC_KEY_LENGTH, ED25519PublicKey};
    use near_sdk::serde_json;

    fn get_actions() -> Vec<Action> {
        vec![
            Action::CreateAccount(CreateAccountAction {}),
            Action::DeployContract(DeployContractAction {
                code: vec![1, 2, 3],
            }),
            Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: "test".to_string(),
                args: vec![4, 5, 6],
                gas: U64(1000000),
                deposit: U128(0),
            })),
            Action::Transfer(TransferAction {
                deposit: U128(1000000000),
            }),
            Action::Stake(Box::new(StakeAction {
                stake: U128(100000000),
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
