use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Deserializer, Serialize};
use serde::de::{self};
use serde::ser::{SerializeTuple, Serializer};
use std::io::{Error, Write};
use std::str::FromStr;

use crate::errors::PublicKeyError;

pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;

#[derive(BorshDeserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Secp256K1PublicKey(pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, Deserialize, BorshDeserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ED25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
}

impl BorshSerialize for PublicKey {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::ED25519(public_key) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
            Self::SECP256K1(public_key) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for PublicKey {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let key_type = <u8 as BorshDeserialize>::deserialize(buf)?;
        match key_type {
            0 => Ok(Self::ED25519(
                <ED25519PublicKey as BorshDeserialize>::deserialize(buf)?,
            )),
            1 => Ok(Self::SECP256K1(
                <Secp256K1PublicKey as BorshDeserialize>::deserialize(buf)?,
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid public key type",
            )),
        }
    }

    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let key_type = u8::deserialize_reader(reader)?;
        match key_type {
            0 => Ok(Self::ED25519(ED25519PublicKey::deserialize_reader(reader)?)),
            1 => Ok(Self::SECP256K1(Secp256K1PublicKey::deserialize_reader(
                reader,
            )?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid public key type",
            )),
        }
    }
}

// From implementations for fixed size arrays
impl From<[u8; SECP256K1_PUBLIC_KEY_LENGTH]> for Secp256K1PublicKey {
    fn from(data: [u8; SECP256K1_PUBLIC_KEY_LENGTH]) -> Self {
        Self(data)
    }
}

impl From<[u8; ED25519_PUBLIC_KEY_LENGTH]> for ED25519PublicKey {
    fn from(data: [u8; ED25519_PUBLIC_KEY_LENGTH]) -> Self {
        Self(data)
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ED25519(key) => write!(f, "ed25519:{}", bs58::encode(key.0).into_string()),
            Self::SECP256K1(key) => write!(f, "secp256k1:{}", bs58::encode(key.0).into_string()),
        }
    }
}

// TryFrom implementations for slices and vectors
impl TryFrom<&[u8]> for PublicKey {
    type Error = PublicKeyError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value.len() {
            ED25519_PUBLIC_KEY_LENGTH => {
                Ok(Self::ED25519(ED25519PublicKey(value.try_into().unwrap())))
            }
            SECP256K1_PUBLIC_KEY_LENGTH => Ok(Self::SECP256K1(Secp256K1PublicKey(
                value.try_into().unwrap(),
            ))),
            _ => Err(PublicKeyError::InvalidLength(value.len())),
        }
    }
}

impl TryFrom<Vec<u8>> for PublicKey {
    type Error = PublicKeyError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<near_openapi_types::PublicKey> for PublicKey {
    type Error = PublicKeyError;
    fn try_from(value: near_openapi_types::PublicKey) -> Result<Self, Self::Error> {
        Self::from_str(&value.0)
    }
}

impl From<PublicKey> for near_openapi_types::PublicKey {
    fn from(val: PublicKey) -> Self {
        near_openapi_types::PublicKey(val.to_string())
    }
}

impl From<near_crypto::PublicKey> for PublicKey {
    fn from(val: near_crypto::PublicKey) -> Self {
        match val {
            near_crypto::PublicKey::ED25519(key) => Self::ED25519(ED25519PublicKey(key.0)),
            near_crypto::PublicKey::SECP256K1(key) => Self::SECP256K1(Secp256K1PublicKey(
                key.as_ref()
                    .try_into()
                    .expect("Invalid SECP256K1 public key"),
            )),
        }
    }
}

impl From<PublicKey> for near_crypto::PublicKey {
    fn from(val: PublicKey) -> Self {
        match val {
            PublicKey::ED25519(key) => near_crypto::PublicKey::ED25519(key.0.into()),
            PublicKey::SECP256K1(key) => near_crypto::PublicKey::SECP256K1(key.0.into()),
        }
    }
}

impl FromStr for PublicKey {
    type Err = PublicKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = s
            .split_once(':')
            .ok_or_else(|| PublicKeyError::InvalidKeyFormat(s.to_string()))?;

        let bytes = bs58::decode(key_data).into_vec()?;

        match key_type {
            "ed25519" => Ok(Self::ED25519(ED25519PublicKey(bytes.try_into()?))),
            "secp256k1" => Ok(Self::SECP256K1(Secp256K1PublicKey(bytes.try_into()?))),
            _ => Err(PublicKeyError::InvalidPrefix(s.to_string())),
        }
    }
}

// Serialization
impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeyOrBytes;

        impl<'de> serde::de::Visitor<'de> for PublicKeyOrBytes {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or byte array representing a public key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(de::Error::custom)
            }

            fn visit_map<V>(self, mut map: V) -> Result<PublicKey, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let key = map
                    .next_key::<String>()?
                    .ok_or_else(|| de::Error::missing_field("key type"))?;
                match key.as_str() {
                    "ED25519" => {
                        let bytes: Vec<u8> = map.next_value()?;
                        PublicKey::try_from(bytes.as_slice()).map_err(de::Error::custom)
                    }
                    "SECP256K1" => {
                        let bytes: Vec<u8> = map.next_value()?;
                        PublicKey::try_from(bytes.as_slice()).map_err(de::Error::custom)
                    }
                    _ => Err(de::Error::unknown_field(&key, &["ED25519", "SECP256K1"])),
                }
            }
        }

        deserializer.deserialize_any(PublicKeyOrBytes)
    }
}

// Big Array
impl Serialize for Secp256K1PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_tuple(SECP256K1_PUBLIC_KEY_LENGTH)?;
        for byte in &self.0 {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Secp256K1PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Secp256K1PublicKeyVisitor;

        impl<'de> serde::de::Visitor<'de> for Secp256K1PublicKeyVisitor {
            type Value = Secp256K1PublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!(
                    "an array of {} bytes",
                    SECP256K1_PUBLIC_KEY_LENGTH
                ))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; SECP256K1_PUBLIC_KEY_LENGTH];
                for (i, byte) in arr.iter_mut().enumerate().take(SECP256K1_PUBLIC_KEY_LENGTH) {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Secp256K1PublicKey(arr))
            }
        }

        deserializer.deserialize_tuple(SECP256K1_PUBLIC_KEY_LENGTH, Secp256K1PublicKeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh;
    use near_sdk::serde_json;

    #[test]
    fn test_public_key_serde_json_serialization() {
        let ed25519_key = PublicKey::ED25519(ED25519PublicKey([8; ED25519_PUBLIC_KEY_LENGTH]));
        let secp256k1_key =
            PublicKey::SECP256K1(Secp256K1PublicKey([9; SECP256K1_PUBLIC_KEY_LENGTH]));

        for key in [ed25519_key, secp256k1_key] {
            let serialized =
                serde_json::to_string(&key).expect("Failed to serialize PublicKey to JSON");

            let deserialized: PublicKey = serde_json::from_str(&serialized)
                .expect("Failed to deserialize PublicKey from JSON");

            assert_eq!(key, deserialized);

            // Check if the JSON string contains the correct key type
            match key {
                PublicKey::ED25519(_) => assert!(serialized.contains("ED25519")),
                PublicKey::SECP256K1(_) => assert!(serialized.contains("SECP256K1")),
            }
        }
    }

    #[test]
    fn test_public_key_borsh_serialization() {
        let ed25519_key = PublicKey::ED25519(ED25519PublicKey([6; ED25519_PUBLIC_KEY_LENGTH]));
        let secp256k1_key =
            PublicKey::SECP256K1(Secp256K1PublicKey([7; SECP256K1_PUBLIC_KEY_LENGTH]));

        for key in [ed25519_key, secp256k1_key] {
            let serialized = borsh::to_vec(&key).expect("Failed to serialize PublicKey");

            let deserialized =
                PublicKey::try_from_slice(&serialized).expect("Failed to deserialize PublicKey");

            assert_eq!(key, deserialized);

            match key {
                PublicKey::ED25519(_) => {
                    assert_eq!(serialized[0], 0);
                    assert_eq!(serialized.len(), 1 + ED25519_PUBLIC_KEY_LENGTH);
                }
                PublicKey::SECP256K1(_) => {
                    assert_eq!(serialized[0], 1);
                    assert_eq!(serialized.len(), 1 + SECP256K1_PUBLIC_KEY_LENGTH);
                }
            }
        }
    }

    #[test]
    fn test_public_key_json_to_borsh_roundtrip() {
        let ed25519_json = r#"
            {
              "signer_public_key": {
                "ED25519": [
                  77, 167, 224, 244, 9, 106, 175, 44, 229, 94, 55, 22, 87, 205, 48, 137,
                  186, 30, 159, 89, 244, 214, 226, 123, 208, 46, 71, 42, 22, 166, 29, 193
                ]
              }
            }"#;

        let secp256k1_json = r#"
            {
              "signer_public_key": {
                "SECP256K1": [
                  77, 167, 224, 244, 9, 106, 175, 44, 229, 94, 55, 22, 87, 205, 48, 137,
                  186, 30, 159, 89, 244, 214, 226, 123, 208, 46, 71, 42, 22, 166, 29, 193,
                  77, 167, 224, 244, 9, 106, 175, 44, 229, 94, 55, 22, 87, 205, 48, 137,
                  186, 30, 159, 89, 244, 214, 226, 123, 208, 46, 71, 42, 22, 166, 29, 193
                ]
              }
            }"#;

        for json in [ed25519_json, secp256k1_json] {
            let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
            let public_key: PublicKey =
                serde_json::from_value(parsed["signer_public_key"].clone()).unwrap();

            // Serialize to Borsh
            let serialized = borsh::to_vec(&public_key).expect("Failed to serialize PublicKey");

            // Deserialize from Borsh
            let deserialized =
                PublicKey::try_from_slice(&serialized).expect("Failed to deserialize PublicKey");

            // Verify the deserialized key matches the original
            assert_eq!(public_key, deserialized);

            let first_bytes_expected_value = 77;
            let last_bytes_expected_value = 193;

            match deserialized {
                PublicKey::ED25519(key) => {
                    assert_eq!(key.0.len(), ED25519_PUBLIC_KEY_LENGTH);
                    assert_eq!(key.0[0], first_bytes_expected_value);
                    assert_eq!(
                        key.0[ED25519_PUBLIC_KEY_LENGTH - 1],
                        last_bytes_expected_value
                    );
                }
                PublicKey::SECP256K1(key) => {
                    assert_eq!(key.0.len(), SECP256K1_PUBLIC_KEY_LENGTH);
                    assert_eq!(key.0[0], first_bytes_expected_value);
                    assert_eq!(
                        key.0[SECP256K1_PUBLIC_KEY_LENGTH - 1],
                        last_bytes_expected_value
                    );
                }
            }
        }
    }
}
