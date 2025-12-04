//! NEP-413: Near Wallet API - support for signMessage method
//!
//! This module provides types and utilities for creating and verifying
//! [NEP-413](https://github.com/near/NEPs/blob/master/neps/nep-0413.md) signed messages.
//!
//! NEP-413 defines a standardized way for NEAR users to sign messages destined to a specific
//! recipient, which is commonly used for authentication in third-party services.
//!
//! # Example
//!
//! ```rust
//! use near_api_types::nep413::{Payload, SignedMessage};
//! use near_api_types::{PublicKey, SecretKey};
//! use std::str::FromStr;
//!
//! // Create a payload to sign
//! let payload = Payload {
//!     message: "Hello NEAR!".to_string(),
//!     nonce: [0u8; 32],
//!     recipient: "myapp.com".to_string(),
//!     callback_url: None,
//! };
//!
//! // Sign the payload (normally done by wallet)
//! let secret_key = SecretKey::from_str(
//!     "ed25519:3tgdk2wPraJzT4nsTuf86UX41xgPNk3MHnq8epARMdBNs29AFEztAuaQ7iHddDfXG9F2RzV1XNQYgJyAyoW51UBB"
//! ).unwrap();
//! let public_key = secret_key.public_key();
//! let hash = payload.compute_hash().unwrap();
//! let signature = secret_key.sign(hash);
//!
//! // Create SignedMessage (as returned by wallet)
//! let signed_message = SignedMessage {
//!     account_id: "alice.near".parse().unwrap(),
//!     public_key: public_key.clone(),
//!     signature: base64::prelude::BASE64_STANDARD.encode(match &signature {
//!         near_api_types::Signature::ED25519(s) => s.to_bytes().to_vec(),
//!         near_api_types::Signature::SECP256K1(s) => s.0.to_vec(),
//!     }),
//!     state: None,
//! };
//!
//! // Verify the signature
//! assert!(signed_message.verify(&payload).unwrap());
//! ```

use base64::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::errors::Nep413Error;
use crate::{crypto::KeyType, CryptoHash, PublicKey, Signature};

/// The NEP-413 discriminant prefix: 2^31 + 413
const NEP413_SIGN_MESSAGE_PREFIX: u32 = (1u32 << 31) + 413;

/// The payload structure for NEP-413 messages (input to signMessage).
///
/// This structure contains the message content and metadata that will be signed.
/// It corresponds to the `Payload` struct defined in NEP-413:
///
/// ```text
/// struct Payload {
///   message: string;
///   nonce: [u8; 32];
///   recipient: string;
///   callbackUrl?: string;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct Payload {
    /// The message that wants to be transmitted.
    pub message: String,
    /// A nonce that uniquely identifies this instance of the message, denoted as a 32 bytes array.
    /// The first 8 bytes can optionally contain a timestamp (ms since epoch) as big-endian uint64.
    pub nonce: [u8; 32],
    /// The recipient to whom the message is destined (e.g. "alice.near" or "myapp.com").
    pub recipient: String,
    /// A callback URL that will be called with the signed message as a query parameter.
    pub callback_url: Option<String>,
}

impl Payload {
    /// Compute the hash that should be signed for this payload.
    ///
    /// According to NEP-413, the signing process is:
    /// 1. Borsh serialize the payload
    /// 2. Prepend the 4-byte Borsh representation of 2^31 + 413 (the NEP-413 tag)
    /// 3. Compute SHA-256 hash of the combined bytes
    /// 4. Sign the hash
    ///
    /// # Errors
    ///
    /// Returns an error if the payload cannot be serialized (should never happen
    /// for valid payloads with String and Option<String> fields).
    pub fn compute_hash(&self) -> Result<CryptoHash, Nep413Error> {
        let mut bytes = NEP413_SIGN_MESSAGE_PREFIX.to_le_bytes().to_vec();
        // Use standard Borsh serialization (same as existing signer)
        borsh::to_writer(&mut bytes, self)?;
        Ok(CryptoHash::hash(&bytes))
    }

    /// Verify a signature against this payload.
    ///
    /// The signature can be provided as:
    /// - Base64 encoded string (no prefix) - as returned by wallets per NEP-413 spec
    /// - Base58 encoded string with prefix like "ed25519:..." or "secp256k1:..."
    ///
    /// # Arguments
    ///
    /// * `signature_str` - The signature as a string (base64 or base58 with prefix)
    /// * `public_key` - The public key to verify against
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid,
    /// or an error if the signature format is invalid.
    pub fn verify_signature(
        &self,
        signature_str: &str,
        public_key: &PublicKey,
    ) -> Result<bool, Nep413Error> {
        let signature = parse_signature(signature_str, public_key.key_type())?;
        let hash = self.compute_hash()?;
        Ok(signature.verify(hash, public_key))
    }

    /// Extract the timestamp from the nonce if present.
    ///
    /// According to NEP-413, the first 8 bytes of the nonce can optionally contain
    /// a timestamp (milliseconds since epoch) as a big-endian uint64.
    ///
    /// Note: This method cannot determine if a timestamp is actually present or if
    /// the first 8 bytes are just random nonce data. The caller should use domain
    /// knowledge to determine if timestamps are expected.
    ///
    /// # Returns
    ///
    /// The timestamp in milliseconds since epoch, interpreted from the first 8 bytes
    /// of the nonce as a big-endian uint64.
    pub fn extract_timestamp_from_nonce(&self) -> u64 {
        // Slicing a [u8; 32] with [..8] always produces exactly 8 bytes,
        // so this conversion is infallible.
        let bytes: [u8; 8] = self.nonce[..8].try_into().unwrap_or_default();
        u64::from_be_bytes(bytes)
    }
}

/// Represents a nonce that can be deserialized from either:
/// - A JSON array of numbers (e.g., `[0, 1, 2, ..., 31]`) - common from JavaScript Uint8Array
/// - A base64-encoded string (e.g., `"AAAAAAA..."`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce(pub [u8; 32]);

impl serde::Serialize for Nonce {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as array of numbers (most compatible with JS)
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(32))?;
        for byte in &self.0 {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for Nonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct NonceVisitor;

        impl<'de> Visitor<'de> for NonceVisitor {
            type Value = Nonce;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 32-byte nonce as either a base64 string or array of numbers")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = BASE64_STANDARD
                    .decode(value)
                    .map_err(|e| de::Error::custom(format!("invalid base64: {}", e)))?;
                let arr: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
                    de::Error::custom(format!("expected 32 bytes, got {}", v.len()))
                })?;
                Ok(Nonce(arr))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 32];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(i, &"32 bytes"))?;
                }
                // Check that there are no extra elements
                if seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(33, &"32 bytes"));
                }
                Ok(Nonce(arr))
            }
        }

        deserializer.deserialize_any(NonceVisitor)
    }
}

impl From<[u8; 32]> for Nonce {
    fn from(arr: [u8; 32]) -> Self {
        Self(arr)
    }
}

impl From<Nonce> for [u8; 32] {
    fn from(nonce: Nonce) -> Self {
        nonce.0
    }
}

/// JSON-compatible version of Payload.
///
/// This is useful when receiving payloads as JSON from frontend applications.
/// The nonce can be either:
/// - A JSON array of 32 numbers (e.g., from JavaScript's `Array.from(Uint8Array)`)
/// - A base64-encoded string
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PayloadJson {
    /// The message that wants to be transmitted.
    pub message: String,
    /// A nonce that uniquely identifies this instance of the message.
    /// Can be either an array of 32 numbers or a base64 string.
    pub nonce: Nonce,
    /// The recipient to whom the message is destined (e.g. "alice.near" or "myapp.com").
    pub recipient: String,
    /// A callback URL that will be called with the signed message as a query parameter.
    #[serde(rename = "callbackUrl", skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

impl From<PayloadJson> for Payload {
    fn from(value: PayloadJson) -> Self {
        Self {
            message: value.message,
            nonce: value.nonce.0,
            recipient: value.recipient,
            callback_url: value.callback_url,
        }
    }
}

impl From<Payload> for PayloadJson {
    fn from(value: Payload) -> Self {
        Self {
            message: value.message,
            nonce: Nonce(value.nonce),
            recipient: value.recipient,
            callback_url: value.callback_url,
        }
    }
}

/// The output structure returned by wallets after signing a message.
///
/// This corresponds to the `SignedMessage` interface defined in NEP-413:
///
/// ```text
/// interface SignedMessage {
///   accountId: string;
///   publicKey: string;
///   signature: string;  // base64 encoded
///   state?: string;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedMessage {
    /// The account name to which the public key corresponds (e.g. "alice.near")
    #[serde(rename = "accountId")]
    pub account_id: near_account_id::AccountId,
    /// The public counterpart of the key used to sign, expressed as a string
    /// with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6E8sCci...")
    #[serde(rename = "publicKey")]
    pub public_key: PublicKey,
    /// The base64 representation of the signature.
    pub signature: String,
    /// Optional state for authentication purposes (applicable to browser wallets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl SignedMessage {
    /// Verify the signature against the given payload.
    ///
    /// This only verifies the cryptographic signature. To verify that the public key
    /// actually belongs to the account, you need to query the NEAR RPC to check if
    /// the account has this public key as an access key.
    ///
    /// # Arguments
    ///
    /// * `payload` - The original payload that was signed
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid,
    /// or an error if the signature format is invalid.
    pub fn verify(&self, payload: &Payload) -> Result<bool, Nep413Error> {
        payload.verify_signature(&self.signature, &self.public_key)
    }
}

/// Parse a signature from either base64 (no prefix) or base58 with prefix format.
///
/// NEP-413 specifies that signatures are returned as base64-encoded strings.
/// However, some implementations may use the NEAR-standard base58 with prefix format
/// (e.g., "ed25519:...").
///
/// # Arguments
///
/// * `signature_str` - The signature string to parse
/// * `expected_key_type` - The expected key type (used for base64 signatures without prefix)
///
/// # Returns
///
/// The parsed `Signature` or an error if parsing fails.
pub fn parse_signature(
    signature_str: &str,
    expected_key_type: KeyType,
) -> Result<Signature, Nep413Error> {
    // Try base58 with prefix first (e.g., "ed25519:..." or "secp256k1:...")
    if signature_str.contains(':') {
        return signature_str
            .parse::<Signature>()
            .map_err(Nep413Error::SignatureParsing);
    }

    // Try base64 (NEP-413 standard format)
    let sig_bytes = BASE64_STANDARD
        .decode(signature_str)
        .map_err(Nep413Error::Base64Decode)?;

    Signature::from_parts(expected_key_type, &sig_bytes).map_err(Nep413Error::SignatureParsing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::secret_key::ED25519SecretKey;
    use crate::SecretKey;
    use std::str::FromStr;

    /// Generate a fresh ED25519 keypair for testing
    fn generate_test_keypair() -> (SecretKey, PublicKey) {
        // Generate random bytes for the secret key
        use sha2::{Digest, Sha256};

        // Use a deterministic seed for reproducible tests
        let seed: [u8; 32] = Sha256::digest(b"test_seed_for_nep413_tests").into();
        let secret_key = SecretKey::ED25519(ED25519SecretKey::from_secret_key(seed));
        let public_key = secret_key.public_key();
        (secret_key, public_key)
    }

    /// Generate a second keypair (different from the first)
    fn generate_second_keypair() -> (SecretKey, PublicKey) {
        use sha2::{Digest, Sha256};

        let seed: [u8; 32] = Sha256::digest(b"second_test_seed_for_nep413").into();
        let secret_key = SecretKey::ED25519(ED25519SecretKey::from_secret_key(seed));
        let public_key = secret_key.public_key();
        (secret_key, public_key)
    }

    fn create_test_payload() -> Payload {
        let mut nonce = [0u8; 32];
        // Put a timestamp in the first 8 bytes (in milliseconds)
        let timestamp: u64 = 1699999999000;
        nonce[..8].copy_from_slice(&timestamp.to_be_bytes());

        Payload {
            message: "Hello NEAR!".to_string(),
            nonce,
            recipient: "myapp.com".to_string(),
            callback_url: None,
        }
    }

    fn sign_payload(payload: &Payload, secret_key: &SecretKey) -> String {
        let hash = payload.compute_hash().expect("test payload serialization");
        let signature = secret_key.sign(hash);
        // Return base64 encoded signature (as per NEP-413 spec)
        match signature {
            Signature::ED25519(sig) => BASE64_STANDARD.encode(sig.to_bytes()),
            Signature::SECP256K1(sig) => BASE64_STANDARD.encode(sig.0),
        }
    }

    #[test]
    fn test_payload_hash_computation() {
        let payload = create_test_payload();
        let hash = payload.compute_hash().expect("test payload serialization");
        // Hash should be deterministic
        let hash2 = payload.compute_hash().expect("test payload serialization");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_extract_timestamp_from_nonce() {
        let payload = create_test_payload();
        let timestamp = payload.extract_timestamp_from_nonce();
        assert_eq!(timestamp, 1699999999000);
    }

    #[test]
    fn test_extract_timestamp_from_zero_nonce() {
        let payload = Payload {
            message: "test".to_string(),
            nonce: [0u8; 32],
            recipient: "test.near".to_string(),
            callback_url: None,
        };
        assert_eq!(payload.extract_timestamp_from_nonce(), 0);
    }

    #[test]
    fn test_signature_verification_base64() {
        let (secret_key, public_key) = generate_test_keypair();

        let payload = create_test_payload();
        let signature_base64 = sign_payload(&payload, &secret_key);

        // Verify with base64 signature
        let result = payload.verify_signature(&signature_base64, &public_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_signature_verification_base58_prefix() {
        let (secret_key, public_key) = generate_test_keypair();

        let payload = create_test_payload();
        let hash = payload.compute_hash().expect("test payload serialization");
        let signature = secret_key.sign(hash);
        let signature_base58 = signature.to_string(); // This gives "ed25519:..."

        // Verify with base58 prefixed signature
        let result = payload.verify_signature(&signature_base58, &public_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_signature_verification_wrong_key() {
        let (secret_key, _) = generate_test_keypair();
        let (_, wrong_public_key) = generate_second_keypair();

        let payload = create_test_payload();
        let signature_base64 = sign_payload(&payload, &secret_key);

        // Verification should fail with wrong key
        let result = payload.verify_signature(&signature_base64, &wrong_public_key);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_signature_verification_tampered_message() {
        let (secret_key, public_key) = generate_test_keypair();

        let payload = create_test_payload();
        let signature_base64 = sign_payload(&payload, &secret_key);

        // Tamper with the message
        let tampered_payload = Payload {
            message: "Tampered message!".to_string(),
            ..payload
        };

        // Verification should fail
        let result = tampered_payload.verify_signature(&signature_base64, &public_key);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_payload_with_callback_url() {
        let payload = Payload {
            message: "Hello NEAR!".to_string(),
            nonce: [0u8; 32],
            recipient: "myapp.com".to_string(),
            callback_url: Some("https://myapp.com/callback".to_string()),
        };

        let (secret_key, public_key) = generate_test_keypair();

        let signature_base64 = sign_payload(&payload, &secret_key);
        let result = payload.verify_signature(&signature_base64, &public_key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_signed_message_verify() {
        let (secret_key, public_key) = generate_test_keypair();

        let payload = create_test_payload();
        let signature_base64 = sign_payload(&payload, &secret_key);

        let signed_message = SignedMessage {
            account_id: "test.near".parse().unwrap(),
            public_key,
            signature: signature_base64,
            state: None,
        };

        let result = signed_message.verify(&payload);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_payload_json_conversion() {
        let nonce_bytes = [1u8; 32];
        let payload = Payload {
            message: "test".to_string(),
            nonce: nonce_bytes,
            recipient: "app.near".to_string(),
            callback_url: Some("https://app.near/cb".to_string()),
        };

        // Convert to JSON version and back
        let json_payload: PayloadJson = payload.clone().into();
        let payload_back: Payload = json_payload.into();

        assert_eq!(payload, payload_back);
    }

    #[test]
    fn test_payload_json_with_array_nonce() {
        // JSON with nonce as array of numbers (typical from JavaScript)
        let json = r#"{
            "message": "Hello",
            "nonce": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31],
            "recipient": "test.near"
        }"#;

        let parsed: PayloadJson = serde_json::from_str(json).unwrap();
        let payload: Payload = parsed.into();

        assert_eq!(payload.message, "Hello");
        assert_eq!(payload.recipient, "test.near");
        assert_eq!(payload.nonce[0], 0);
        assert_eq!(payload.nonce[31], 31);
        assert!(payload.callback_url.is_none());
    }

    #[test]
    fn test_payload_json_with_base64_nonce() {
        // JSON with nonce as base64 string
        let json = r#"{
            "message": "Hello",
            "nonce": "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=",
            "recipient": "test.near",
            "callbackUrl": "https://test.near/cb"
        }"#;

        let parsed: PayloadJson = serde_json::from_str(json).unwrap();
        let payload: Payload = parsed.into();

        assert_eq!(payload.message, "Hello");
        assert_eq!(payload.recipient, "test.near");
        assert_eq!(payload.nonce[0], 0);
        assert_eq!(payload.nonce[31], 31);
        assert_eq!(
            payload.callback_url,
            Some("https://test.near/cb".to_string())
        );
    }

    #[test]
    fn test_payload_json_serialization_produces_array() {
        let payload = Payload {
            message: "Hello".to_string(),
            nonce: [0u8; 32],
            recipient: "test.near".to_string(),
            callback_url: None,
        };

        let json_payload: PayloadJson = payload.into();
        let json = serde_json::to_string(&json_payload).unwrap();

        // Should serialize nonce as array, not base64
        assert!(json.contains("[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]"));
        assert!(json.contains("\"message\":\"Hello\""));
        assert!(json.contains("\"recipient\":\"test.near\""));
    }

    #[test]
    fn test_invalid_nonce_length_in_json() {
        // JSON with wrong nonce length
        let json = r#"{
            "message": "Hello",
            "nonce": [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
            "recipient": "test.near"
        }"#;

        let result: Result<PayloadJson, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_signed_message_json_serialization() {
        let (secret_key, public_key) = generate_test_keypair();
        let payload = create_test_payload();
        let signature_base64 = sign_payload(&payload, &secret_key);

        let signed_message = SignedMessage {
            account_id: "alice.near".parse().unwrap(),
            public_key,
            signature: signature_base64,
            state: Some("auth_state_123".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&signed_message).unwrap();

        // Check that field names match NEP-413 spec
        assert!(json.contains("\"accountId\""));
        assert!(json.contains("\"publicKey\""));
        assert!(json.contains("\"signature\""));
        assert!(json.contains("\"state\""));

        // Deserialize back
        let parsed: SignedMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(signed_message, parsed);
    }

    #[test]
    fn test_invalid_signature_format() {
        let payload = create_test_payload();
        let public_key =
            PublicKey::from_str("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").unwrap();

        // Invalid base64
        let result = payload.verify_signature("not-valid-base64!!!", &public_key);
        assert!(result.is_err());
    }

    /// Test compatibility with the existing signer implementation in api/src/signer/mod.rs
    /// Uses the same test data from the existing NEP-413 tests
    #[test]
    fn test_compatibility_with_existing_signer_without_callback() {
        // Test data from api/src/signer/mod.rs nep_413_tests::without_callback_url
        let nonce: [u8; 32] = BASE64_STANDARD
            .decode("KNV0cOpvJ50D5vfF9pqWom8wo2sliQ4W+Wa7uZ3Uk6Y=")
            .unwrap()
            .try_into()
            .unwrap();

        let payload = Payload {
            message: "Hello NEAR!".to_string(),
            nonce,
            recipient: "example.near".to_string(),
            callback_url: None,
        };

        // The expected signature from the existing test (base64 encoded)
        let expected_signature = "NnJgPU1Ql7ccRTITIoOVsIfElmvH1RV7QAT4a9Vh6ShCOnjIzRwxqX54JzoQ/nK02p7VBMI2vJn48rpImIJwAw==";

        // The public key derived from seed phrase "fatal edge jacket cash hard pass gallery fabric whisper size rain biology"
        // using DEFAULT_HD_PATH "m/44'/397'/0'"
        let public_key =
            PublicKey::from_str("ed25519:2RM3EotCzEiVobm6aMjaup43k8cFffR4KHFtrqbZ79Qy").unwrap();

        // Verify the signature
        let result = payload.verify_signature(expected_signature, &public_key);
        assert!(
            result.is_ok(),
            "Signature verification failed: {:?}",
            result
        );
        assert!(result.unwrap(), "Signature should be valid");
    }

    #[test]
    fn test_compatibility_with_existing_signer_with_callback() {
        // Test data from api/src/signer/mod.rs nep_413_tests::with_callback_url
        let nonce: [u8; 32] = BASE64_STANDARD
            .decode("KNV0cOpvJ50D5vfF9pqWom8wo2sliQ4W+Wa7uZ3Uk6Y=")
            .unwrap()
            .try_into()
            .unwrap();

        let payload = Payload {
            message: "Hello NEAR!".to_string(),
            nonce,
            recipient: "example.near".to_string(),
            callback_url: Some("http://localhost:3000".to_string()),
        };

        // The expected signature from the existing test (base64 encoded)
        let expected_signature = "zzZQ/GwAjrZVrTIFlvmmQbDQHllfzrr8urVWHaRt5cPfcXaCSZo35c5LDpPpTKivR6BxLyb3lcPM0FfCW5lcBQ==";

        // The public key derived from seed phrase "fatal edge jacket cash hard pass gallery fabric whisper size rain biology"
        let public_key =
            PublicKey::from_str("ed25519:2RM3EotCzEiVobm6aMjaup43k8cFffR4KHFtrqbZ79Qy").unwrap();

        // Verify the signature
        let result = payload.verify_signature(expected_signature, &public_key);
        assert!(
            result.is_ok(),
            "Signature verification failed: {:?}",
            result
        );
        assert!(result.unwrap(), "Signature should be valid");
    }

    #[test]
    fn test_full_roundtrip_sign_and_verify() {
        // Generate a fresh keypair
        let (secret_key, public_key) = generate_test_keypair();

        // Create payload
        let payload = Payload {
            message: "Authenticate me!".to_string(),
            nonce: {
                let mut n = [0u8; 32];
                // Timestamp in first 8 bytes
                let ts: u64 = 1700000000000;
                n[..8].copy_from_slice(&ts.to_be_bytes());
                // Random-ish data in rest
                for (i, byte) in n[8..].iter_mut().enumerate() {
                    *byte = (i as u8).wrapping_mul(7);
                }
                n
            },
            recipient: "myapp.example.com".to_string(),
            callback_url: Some("https://myapp.example.com/auth/callback".to_string()),
        };

        // Sign (simulating wallet behavior)
        let hash = payload.compute_hash().expect("test payload serialization");
        let signature = secret_key.sign(hash);
        let signature_base64 = match &signature {
            Signature::ED25519(sig) => BASE64_STANDARD.encode(sig.to_bytes()),
            Signature::SECP256K1(sig) => BASE64_STANDARD.encode(sig.0),
        };

        // Create SignedMessage (as wallet would return)
        let signed_message = SignedMessage {
            account_id: "user.near".parse().unwrap(),
            public_key,
            signature: signature_base64,
            state: Some("csrf_token_abc123".to_string()),
        };

        // Verify (simulating backend behavior)
        let is_valid = signed_message.verify(&payload).unwrap();
        assert!(is_valid, "Full roundtrip verification should succeed");

        // Also test extraction of timestamp from nonce
        assert_eq!(payload.extract_timestamp_from_nonce(), 1700000000000);
    }
}
