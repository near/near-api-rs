use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use crate::errors::{DataConversionError, KeyTypeError};

pub mod public_key;
pub mod secret_key;
pub mod signature;

pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
pub const COMPONENT_SIZE: usize = 32;
pub const SECP256K1_SIGNATURE_LENGTH: usize = 65;

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub enum KeyType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(match self {
            Self::ED25519 => "ed25519",
            Self::SECP256K1 => "secp256k1",
        })
    }
}

impl FromStr for KeyType {
    type Err = KeyTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let lowercase_key_type = value.to_ascii_lowercase();
        match lowercase_key_type.as_str() {
            "ed25519" => Ok(Self::ED25519),
            "secp256k1" => Ok(Self::SECP256K1),
            _ => Err(KeyTypeError::InvalidKeyFormat(
                lowercase_key_type.to_string(),
            )),
        }
    }
}

impl TryFrom<u8> for KeyType {
    type Error = KeyTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ED25519),
            1 => Ok(Self::SECP256K1),
            unknown_key_type => Err(KeyTypeError::InvalidKeyTypeByteIndex(unknown_key_type)),
        }
    }
}

fn split_key_type_data(value: &str) -> Result<(KeyType, &str), DataConversionError> {
    if let Some((prefix, key_data)) = value.split_once(':') {
        Ok((KeyType::from_str(prefix)?, key_data))
    } else {
        // If there is no prefix then we Default to ED25519.
        Ok((KeyType::ED25519, value))
    }
}

#[cfg(test)]
mod tests {
    use crate::CryptoHash;

    use super::{public_key::PublicKey, secret_key::SecretKey, signature::Signature, KeyType};

    #[test]
    fn signature_verify_fuzzer() {
        bolero::check!().with_type().for_each(
            |(key_type, sign, data, public_key): &(KeyType, [u8; 65], [u8; 32], PublicKey)| {
                let signature = match key_type {
                    KeyType::ED25519 => {
                        Signature::from_parts(KeyType::ED25519, &sign[..64]).unwrap()
                    }
                    KeyType::SECP256K1 => {
                        Signature::from_parts(KeyType::SECP256K1, &sign[..65]).unwrap()
                    }
                };
                let _ = signature.verify(CryptoHash(*data), *public_key);
            },
        );
    }

    #[test]
    fn regression_signature_verification_originally_failed() {
        let signature = Signature::from_parts(KeyType::SECP256K1, &[4; 65]).unwrap();
        let _ = signature.verify(CryptoHash([0; 32]), PublicKey::empty(KeyType::SECP256K1));
    }

    #[test]
    fn test_invalid_data() {
        // cspell:disable-next-line
        let invalid = "\"secp256k1:2xVqteU8PWhadHTv99TGh3bSf\"";
        assert!(serde_json::from_str::<PublicKey>(invalid).is_err());
        assert!(serde_json::from_str::<SecretKey>(invalid).is_err());
        assert!(serde_json::from_str::<Signature>(invalid).is_err());
    }
}
