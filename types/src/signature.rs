use borsh::{BorshDeserialize, BorshSerialize};
use bs58;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Debug, str::FromStr};

use crate::errors::{DataConversionError, SignatureError};

pub const COMPONENT_SIZE: usize = 32;
pub const SECP256K1_SIGNATURE_LENGTH: usize = 65;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum Signature {
    ED25519(ED25519Signature),
    SECP256K1(Secp256K1Signature),
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct ED25519Signature {
    pub r: ComponentBytes,
    pub s: ComponentBytes,
}

/// Size of an `R` or `s` component of an Ed25519 signature when serialized as bytes.
pub type ComponentBytes = [u8; COMPONENT_SIZE];

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct Secp256K1Signature(pub [u8; SECP256K1_SIGNATURE_LENGTH]);

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::ED25519(sig) => {
                let mut bytes = Vec::with_capacity(COMPONENT_SIZE * 2);
                bytes.extend_from_slice(&sig.r);
                bytes.extend_from_slice(&sig.s);

                let encoded = bs58::encode(&bytes).into_string();
                serializer.serialize_str(&format!("ed25519:{encoded}"))
            }
            Self::SECP256K1(sig) => {
                let encoded = bs58::encode(&sig.0).into_string();
                serializer.serialize_str(&format!("secp256k1:{encoded}"))
            }
        }
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ED25519(sig) => write!(f, "ed25519:{}", bs58::encode(&sig.r).into_string()),
            Self::SECP256K1(sig) => write!(f, "secp256k1:{}", bs58::encode(&sig.0).into_string()),
        }
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() == COMPONENT_SIZE * 2 {
            let signature = ED25519Signature {
                r: value[0..COMPONENT_SIZE].try_into().unwrap(),
                s: value[COMPONENT_SIZE..].try_into().unwrap(),
            };
            Ok(Signature::ED25519(signature))
        } else if value.len() == SECP256K1_SIGNATURE_LENGTH {
            let signature = Secp256K1Signature(value.try_into().unwrap());
            Ok(Signature::SECP256K1(signature))
        } else {
            Err("Invalid signature length")
        }
    }
}

impl FromStr for Signature {
    type Err = DataConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key_type, sig_data) = s.split_at(
            s.find(':')
                .ok_or_else(|| SignatureError::InvalidSignatureFormat(s.to_string()))?,
        );
        let sig_data = &sig_data[1..]; // Skip the colon

        match key_type {
            "ed25519" => {
                let bytes = bs58::decode(sig_data).into_vec()?;

                let signature = ED25519Signature {
                    r: bytes[0..COMPONENT_SIZE]
                        .try_into()
                        .map_err(|_| DataConversionError::IncorrectLength(bytes.len()))?,
                    s: bytes[COMPONENT_SIZE..]
                        .try_into()
                        .map_err(|_| DataConversionError::IncorrectLength(bytes.len()))?,
                };
                Ok(Self::ED25519(signature))
            }
            "secp256k1" => {
                let bytes = bs58::decode(sig_data).into_vec()?;

                if bytes.len() != SECP256K1_SIGNATURE_LENGTH {
                    return Err(DataConversionError::IncorrectLength(bytes.len()));
                }

                let mut array = [0u8; SECP256K1_SIGNATURE_LENGTH];
                array.copy_from_slice(&bytes);
                Ok(Self::SECP256K1(Secp256K1Signature(array)))
            }
            _ => Err(SignatureError::InvalidSignatureFormat(s.to_string()))?,
        }
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Signature::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl From<near_crypto::Signature> for Signature {
    fn from(signature: near_crypto::Signature) -> Self {
        borsh::from_slice(&borsh::to_vec(&signature).expect("Failed to serialize signature"))
            .expect("Failed to deserialize signature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserialize_ed25519_signature() {
        let serialized = "\"ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj\"";
        let deserialized: Signature = serde_json::from_str(serialized).unwrap();

        let decoded = bs58::decode("3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj")
            .into_vec()
            .unwrap();

        let expected = Signature::ED25519(ED25519Signature {
            r: decoded[0..COMPONENT_SIZE].try_into().unwrap(),
            s: decoded[COMPONENT_SIZE..].try_into().unwrap(),
        });

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_deserialize_secp256k1_signature() {
        let serialized = "\"secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6\"";
        let deserialized: Signature = serde_json::from_str(serialized).unwrap();

        let decoded = bs58::decode("5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6")
            .into_vec()
            .unwrap();

        let expected = Signature::SECP256K1(Secp256K1Signature(decoded.try_into().unwrap()));

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_deserialize_with_invalid_data() {
        let invalid = "\"secp256k1:2xVqteU8PWhadHTv99TGh3bSf\"";

        assert!(serde_json::from_str::<Signature>(invalid).is_err());
    }

    #[test]
    fn test_deserialize_with_valid_data_from_str() {
        let invalid = "secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6";
        assert!(Signature::from_str(invalid).is_ok());
    }

    #[test]
    fn test_serialize_ed25519_signature() {
        // Decode the base58 signature to get the components r and s
        let decoded = bs58::decode("3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj")
            .into_vec()
            .unwrap();

        let signature = Signature::ED25519(ED25519Signature {
            r: decoded[0..COMPONENT_SIZE].try_into().unwrap(),
            s: decoded[COMPONENT_SIZE..].try_into().unwrap(),
        });

        let serialized = serde_json::to_string(&signature).unwrap();
        let expected = "\"ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj\"";

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_serialize_secp256k1_signature() {
        // Decode the base58 signature to get the array of bytes
        let decoded = bs58::decode("5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6")
            .into_vec()
            .unwrap();

        let signature = Signature::SECP256K1(Secp256K1Signature(decoded.try_into().unwrap()));
        let serialized = serde_json::to_string(&signature).unwrap();
        let expected = "\"secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6\"";

        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_borsh_serialize_deserialize_ed25519() {
        let decoded = bs58::decode("3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj")
            .into_vec()
            .unwrap();

        let signature = Signature::ED25519(ED25519Signature {
            r: decoded[0..COMPONENT_SIZE].try_into().unwrap(),
            s: decoded[COMPONENT_SIZE..].try_into().unwrap(),
        });

        let serialized = borsh::to_vec(&signature).unwrap();
        let deserialized: Signature = borsh::BorshDeserialize::try_from_slice(&serialized).unwrap();

        assert_eq!(signature, deserialized);
    }

    #[test]
    fn test_borsh_serialize_deserialize_secp256k1() {
        let decoded = bs58::decode("5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6")
            .into_vec()
            .unwrap();

        let signature = Signature::SECP256K1(Secp256K1Signature(decoded.try_into().unwrap()));

        let serialized = borsh::to_vec(&signature).unwrap();
        let deserialized: Signature = borsh::BorshDeserialize::try_from_slice(&serialized).unwrap();

        assert_eq!(signature, deserialized);
    }
}
