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
                .map(|action| {
                    NonDelegateAction::try_from(Action::try_from(action.0)?)
                        .map_err(|_| DataConversionError::DelegateActionNotSupported)
                })
                .collect::<Result<Vec<_>, DataConversionError>>()?,
            nonce,
            max_block_height,
            public_key: public_key.try_into()?,
        })
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

/// A wrapper around [near_primitives::action::delegate::SignedDelegateAction] that allows for easy serialization and deserialization as base64 string
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
