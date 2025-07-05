use near_primitives::borsh;

use crate::errors::SignedDelegateActionError;

/// A wrapper around [near_primitives::action::delegate::SignedDelegateAction] that allows for easy serialization and deserialization as base64 string
///
/// The type implements [std::str::FromStr] and [std::fmt::Display] to serialize and deserialize the type as base64 string
#[derive(Debug, Clone)]
pub struct SignedDelegateActionAsBase64 {
    /// The inner signed delegate action
    pub inner: near_primitives::action::delegate::SignedDelegateAction,
}

impl std::str::FromStr for SignedDelegateActionAsBase64 {
    type Err = SignedDelegateActionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: borsh::from_slice(
                &near_primitives::serialize::from_base64(s)
                    .map_err(|_| SignedDelegateActionError::Base64DecodingError)?,
            )?,
        })
    }
}

impl std::fmt::Display for SignedDelegateActionAsBase64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base64_signed_delegate_action = near_primitives::serialize::to_base64(
            &borsh::to_vec(&self.inner)
                .expect("Signed Delegate Action serialization to borsh is not expected to fail"),
        );
        write!(f, "{base64_signed_delegate_action}")
    }
}

impl From<near_primitives::action::delegate::SignedDelegateAction>
    for SignedDelegateActionAsBase64
{
    fn from(value: near_primitives::action::delegate::SignedDelegateAction) -> Self {
        Self { inner: value }
    }
}
