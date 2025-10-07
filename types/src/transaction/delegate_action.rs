use std::str::FromStr;

use base64::{Engine, prelude::BASE64_STANDARD};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    AccountId, Action, BlockHeight, Nonce, PublicKey, Signature, errors::DataConversionError,
};

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Eq)]
pub struct NonDelegateAction(Action);

impl TryFrom<Action> for NonDelegateAction {
    type Error = ();
    fn try_from(action: Action) -> Result<Self, Self::Error> {
        if let Action::Delegate(_) = action {
            return Err(());
        }
        Ok(Self(action))
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegateAction {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<NonDelegateAction>,
    pub nonce: Nonce,
    pub max_block_height: BlockHeight,
    pub public_key: PublicKey,
}

impl TryFrom<near_openapi_types::DelegateAction> for DelegateAction {
    type Error = DataConversionError;
    fn try_from(value: near_openapi_types::DelegateAction) -> Result<Self, Self::Error> {
        let near_openapi_types::DelegateAction {
            sender_id,
            receiver_id,
            actions,
            nonce,
            max_block_height,
            public_key,
        } = value;

        Ok(Self {
            sender_id,
            receiver_id,
            actions: actions
                .into_iter()
                .map(NonDelegateAction::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            nonce,
            max_block_height,
            public_key: public_key.try_into()?,
        })
    }
}

impl TryFrom<near_openapi_types::NonDelegateAction> for NonDelegateAction {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::NonDelegateAction) -> Result<Self, Self::Error> {
        match val {
            near_openapi_types::NonDelegateAction::DeterministicStateInit(
                deterministic_state_init,
            ) => Ok(Self(Action::DeterministicStateInit(Box::new(
                deterministic_state_init.try_into()?,
            )))),
            near_openapi_types::NonDelegateAction::CreateAccount(create_account_action) => {
                Ok(Self(Action::CreateAccount(create_account_action.into())))
            }
            near_openapi_types::NonDelegateAction::DeployContract(deploy_contract_action) => Ok(
                Self(Action::DeployContract(deploy_contract_action.try_into()?)),
            ),
            near_openapi_types::NonDelegateAction::FunctionCall(function_call_action) => Ok(Self(
                Action::FunctionCall(Box::new(function_call_action.try_into()?)),
            )),
            near_openapi_types::NonDelegateAction::Transfer(transfer_action) => {
                Ok(Self(Action::Transfer(transfer_action.try_into()?)))
            }
            near_openapi_types::NonDelegateAction::Stake(stake_action) => {
                Ok(Self(Action::Stake(Box::new(stake_action.try_into()?))))
            }
            near_openapi_types::NonDelegateAction::AddKey(add_key_action) => {
                Ok(Self(Action::AddKey(Box::new(add_key_action.try_into()?))))
            }
            near_openapi_types::NonDelegateAction::DeleteKey(delete_key_action) => Ok(Self(
                Action::DeleteKey(Box::new(delete_key_action.try_into()?)),
            )),
            near_openapi_types::NonDelegateAction::DeleteAccount(delete_account_action) => {
                Ok(Self(Action::DeleteAccount(delete_account_action.into())))
            }
            near_openapi_types::NonDelegateAction::DeployGlobalContract(
                deploy_global_contract_action,
            ) => Ok(Self(Action::DeployGlobalContract(
                deploy_global_contract_action.try_into()?,
            ))),
            near_openapi_types::NonDelegateAction::UseGlobalContract(
                use_global_contract_action,
            ) => Ok(Self(Action::UseGlobalContract(Box::new(
                use_global_contract_action.try_into()?,
            )))),
        }
    }
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Signature,
}

impl TryFrom<near_openapi_types::SignedDelegateAction> for SignedDelegateAction {
    type Error = DataConversionError;
    fn try_from(value: near_openapi_types::SignedDelegateAction) -> Result<Self, Self::Error> {
        let near_openapi_types::SignedDelegateAction {
            delegate_action,
            signature,
        } = value;
        Ok(Self {
            delegate_action: delegate_action.try_into()?,
            signature: Signature::from_str(&signature)?,
        })
    }
}

/// A wrapper around [crate::transaction::delegate_action::SignedDelegateAction] that allows for easy serialization and deserialization as base64 string
///
/// The type implements [std::str::FromStr] and [std::fmt::Display] to serialize and deserialize the type as base64 string
#[derive(Debug, Clone)]
pub struct SignedDelegateActionAsBase64 {
    /// The inner signed delegate action
    pub inner: SignedDelegateAction,
}

impl std::str::FromStr for SignedDelegateActionAsBase64 {
    type Err = DataConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: borsh::from_slice(&bs58::decode(s).into_vec()?)?,
        })
    }
}

impl std::fmt::Display for SignedDelegateActionAsBase64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base64_signed_delegate_action = BASE64_STANDARD.encode(
            borsh::to_vec(&self.inner)
                .expect("Signed Delegate Action serialization to borsh is not expected to fail"),
        );
        write!(f, "{base64_signed_delegate_action}")
    }
}

impl From<SignedDelegateAction> for SignedDelegateActionAsBase64 {
    fn from(value: SignedDelegateAction) -> Self {
        Self { inner: value }
    }
}
