//! NEP-413 message verification utilities
//!
//! This module provides functions to verify [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md)
//! signed messages. NEP-413 is used for off-chain authentication where a user signs a message
//! with their NEAR account key.
//!
//! # Example
//!
//! ```rust,no_run
//! use near_api::{verify::verify_signed_message, NetworkConfig};
//! use near_api::types::nep413::{Payload, SignedMessage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Received from frontend
//! let signed_message: SignedMessage = serde_json::from_str(r#"{
//!     "accountId": "alice.near",
//!     "publicKey": "ed25519:...",
//!     "signature": "...",
//!     "state": "csrf_token"
//! }"#)?;
//!
//! let payload = Payload {
//!     message: "Login to MyApp".to_string(),
//!     nonce: [0u8; 32],
//!     recipient: "myapp.com".to_string(),
//!     callback_url: None,
//! };
//!
//! // Verify signature AND that the public key belongs to the account
//! let is_valid = verify_signed_message(&signed_message, &payload, &NetworkConfig::mainnet()).await?;
//!
//! if is_valid {
//!     println!("User {} authenticated successfully!", signed_message.account_id);
//! }
//! # Ok(())
//! # }
//! ```

use near_api_types::nep413::{Payload, SignedMessage};
use near_api_types::AccessKeyPermission;

use crate::{config::NetworkConfig, errors::Nep413VerificationError, Account};

/// Verify a NEP-413 signed message.
///
/// This function performs two verifications:
/// 1. Cryptographic signature verification - ensures the signature is valid for the payload
/// 2. Account key verification - queries the NEAR RPC to verify the public key belongs to the account
///    and is a **full access key** (NEP-413 requires full access keys for signing)
///
/// # Arguments
///
/// * `signed_message` - The signed message received from the wallet
/// * `payload` - The original payload that was signed
/// * `network` - The network configuration to use for RPC queries
///
/// # Returns
///
/// Returns `Ok(true)` if both verifications pass, `Ok(false)` if the signature is invalid,
/// or an error if verification fails (e.g., network error, public key not found, not a full access key).
///
/// # Example
///
/// ```rust,no_run
/// use near_api::{verify::verify_signed_message, NetworkConfig};
/// use near_api::types::nep413::{Payload, SignedMessage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let signed_message = SignedMessage {
///     account_id: "alice.near".parse()?,
///     public_key: "ed25519:...".parse()?,
///     signature: "base64_signature".to_string(),
///     state: None,
/// };
///
/// let payload = Payload {
///     message: "Hello".to_string(),
///     nonce: [0u8; 32],
///     recipient: "myapp.com".to_string(),
///     callback_url: None,
/// };
///
/// let is_valid = verify_signed_message(&signed_message, &payload, &NetworkConfig::mainnet()).await?;
/// # Ok(())
/// # }
/// ```
pub async fn verify_signed_message(
    signed_message: &SignedMessage,
    payload: &Payload,
    network: &NetworkConfig,
) -> Result<bool, Nep413VerificationError> {
    // Step 1: Verify the cryptographic signature
    let signature_valid = signed_message
        .verify(payload)
        .map_err(Nep413VerificationError::SignatureVerification)?;

    if !signature_valid {
        return Ok(false);
    }

    // Step 2: Verify the public key belongs to the account AND is a full access key
    verify_full_access_key(
        &signed_message.account_id,
        &signed_message.public_key,
        network,
    )
    .await
}

/// Verify that a public key is a full access key for an account.
///
/// This queries the NEAR RPC to check if the given public key is registered
/// as a **full access** key for the account. Per NEP-413 spec, messages must
/// be signed with full access keys only.
///
/// # Arguments
///
/// * `account_id` - The account ID to check
/// * `public_key` - The public key to verify
/// * `network` - The network configuration to use for RPC queries
///
/// # Returns
///
/// Returns `Ok(true)` if the public key is a full access key for the account,
/// `Ok(false)` if key not found, or an error if the query fails or key is not full access.
pub async fn verify_full_access_key(
    account_id: &near_api_types::AccountId,
    public_key: &near_api_types::PublicKey,
    network: &NetworkConfig,
) -> Result<bool, Nep413VerificationError> {
    // Query the specific access key directly
    let key_result = Account(account_id.clone())
        .access_key(public_key.clone())
        .fetch_from(network)
        .await;

    match key_result {
        Ok(data) => {
            // Check if it's a full access key (NEP-413 requirement)
            match data.data.permission {
                AccessKeyPermission::FullAccess => Ok(true),
                AccessKeyPermission::FunctionCall(_) => {
                    Err(Nep413VerificationError::NotFullAccessKey)
                }
            }
        }
        Err(e) => {
            // Check if the error indicates the key wasn't found vs a real error
            let error_str = format!("{:?}", e);
            if error_str.contains("UnknownAccessKey")
                || error_str.contains("does not exist")
                || error_str.contains("AccessKeyDoesNotExist")
            {
                Ok(false) // Key not found
            } else {
                Err(Nep413VerificationError::RpcError(Box::new(e)))
            }
        }
    }
}

/// Verify that a public key is associated with an account (any key type).
///
/// This is a simpler check that doesn't require the key to be a full access key.
/// For NEP-413 verification, use [`verify_full_access_key`] instead.
///
/// # Arguments
///
/// * `account_id` - The account ID to check
/// * `public_key` - The public key to verify
/// * `network` - The network configuration to use for RPC queries
///
/// # Returns
///
/// Returns `Ok(true)` if the public key is associated with the account (any key type),
/// `Ok(false)` if not found, or an error if the query fails.
pub async fn verify_public_key_belongs_to_account(
    account_id: &near_api_types::AccountId,
    public_key: &near_api_types::PublicKey,
    network: &NetworkConfig,
) -> Result<bool, Nep413VerificationError> {
    // Query the specific access key directly (more efficient than listing all keys)
    let key_result = Account(account_id.clone())
        .access_key(public_key.clone())
        .fetch_from(network)
        .await;

    match key_result {
        Ok(_) => Ok(true), // Key exists
        Err(e) => {
            // Check if the error indicates the key wasn't found vs a real error
            let error_str = format!("{:?}", e);
            if error_str.contains("UnknownAccessKey")
                || error_str.contains("does not exist")
                || error_str.contains("AccessKeyDoesNotExist")
            {
                Ok(false) // Key not found
            } else {
                Err(Nep413VerificationError::RpcError(Box::new(e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_api_types::nep413::PayloadJson;

    #[test]
    fn test_payload_json_deserialization_with_array_nonce() {
        // Simulate JSON received from a JavaScript frontend
        let json = r#"{
            "message": "Login to MyApp at 2024-01-15T10:30:00Z",
            "nonce": [40,213,116,112,234,111,39,157,3,229,247,197,246,154,150,162,111,48,163,107,36,56,22,249,102,187,185,157,212,147,166],
            "recipient": "myapp.com",
            "callbackUrl": "https://myapp.com/auth/callback"
        }"#;

        // This should fail because nonce has wrong length (31 bytes)
        let result: Result<PayloadJson, _> = serde_json::from_str(json);
        assert!(result.is_err());

        // With correct 32-byte nonce
        let json_correct = r#"{
            "message": "Login to MyApp",
            "nonce": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31],
            "recipient": "myapp.com"
        }"#;

        let parsed: PayloadJson = serde_json::from_str(json_correct).unwrap();
        let payload: Payload = parsed.into();
        assert_eq!(payload.message, "Login to MyApp");
        assert_eq!(payload.recipient, "myapp.com");
        assert_eq!(payload.nonce[0], 0);
        assert_eq!(payload.nonce[31], 31);
    }

    #[test]
    fn test_signed_message_json_deserialization() {
        let json = r#"{
            "accountId": "alice.testnet",
            "publicKey": "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
            "signature": "NnJgPU1Ql7ccRTITIoOVsIfElmvH1RV7QAT4a9Vh6ShCOnjIzRwxqX54JzoQ/nK02p7VBMI2vJn48rpImIJwAw==",
            "state": "csrf_token_123"
        }"#;

        let signed_message: SignedMessage = serde_json::from_str(json).unwrap();
        assert_eq!(signed_message.account_id.to_string(), "alice.testnet");
        assert_eq!(signed_message.state, Some("csrf_token_123".to_string()));
    }
}
