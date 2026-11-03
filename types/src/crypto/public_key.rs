use std::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    io::{Error, ErrorKind, Read, Write},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    crypto::{
        KeyType, ML_DSA_65_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH, split_key_type_data,
    },
    errors::DataConversionError,
};

/// Public key container supporting different curves.
///
/// Note that this type is not `Copy`: the ML-DSA-65 variant owns a heap-allocated 1952-byte key.
/// Use `.clone()` where a copy is needed.
#[derive(Clone, PartialEq, PartialOrd, Ord, Eq)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
    /// FIPS 204 ML-DSA-65 post-quantum public key (1952 bytes).
    MLDSA65(MlDsa65PublicKey),
}

impl PublicKey {
    // `is_empty` always returns false, so there is no point in adding it
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        const ED25519_LEN: usize = ed25519_dalek::PUBLIC_KEY_LENGTH + 1;
        match self {
            Self::ED25519(_) => ED25519_LEN,
            Self::SECP256K1(_) => 65,
            Self::MLDSA65(_) => 1 + ML_DSA_65_PUBLIC_KEY_LENGTH,
        }
    }

    // Not `const` because the ML-DSA-65 key is heap-allocated.
    pub fn empty(key_type: KeyType) -> Self {
        match key_type {
            KeyType::ED25519 => {
                Self::ED25519(ED25519PublicKey([0u8; ed25519_dalek::PUBLIC_KEY_LENGTH]))
            }
            KeyType::SECP256K1 => Self::SECP256K1(Secp256K1PublicKey([0u8; 64])),
            KeyType::MLDSA65 => Self::MLDSA65(MlDsa65PublicKey(Box::new(
                [0u8; ML_DSA_65_PUBLIC_KEY_LENGTH],
            ))),
        }
    }

    pub const fn key_type(&self) -> KeyType {
        match self {
            Self::ED25519(_) => KeyType::ED25519,
            Self::SECP256K1(_) => KeyType::SECP256K1,
            Self::MLDSA65(_) => KeyType::MLDSA65,
        }
    }

    pub const fn key_data(&self) -> &[u8] {
        match self {
            Self::ED25519(key) => &key.0,
            Self::SECP256K1(key) => &key.0,
            Self::MLDSA65(key) => &*key.0,
        }
    }

    pub const fn unwrap_as_ed25519(&self) -> &ED25519PublicKey {
        match self {
            Self::ED25519(key) => key,
            Self::SECP256K1(_) | Self::MLDSA65(_) => panic!(),
        }
    }

    pub const fn unwrap_as_secp256k1(&self) -> &Secp256K1PublicKey {
        match self {
            Self::SECP256K1(key) => key,
            Self::ED25519(_) | Self::MLDSA65(_) => panic!(),
        }
    }
}

impl TryFrom<near_openapi_types::PublicKey> for PublicKey {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::PublicKey) -> Result<Self, Self::Error> {
        Self::from_str(&val.0)
    }
}

impl From<PublicKey> for near_openapi_types::PublicKey {
    fn from(val: PublicKey) -> Self {
        Self(val.to_string())
    }
}

// This `Hash` implementation is safe since it retains the property
// `k1 == k2 ⇒ hash(k1) == hash(k2)`.
impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::ED25519(public_key) => {
                state.write_u8(0u8);
                state.write(&public_key.0);
            }
            Self::SECP256K1(public_key) => {
                state.write_u8(1u8);
                state.write(&public_key.0);
            }
            Self::MLDSA65(public_key) => {
                state.write_u8(2u8);
                state.write(&*public_key.0);
            }
        }
    }
}

impl Display for PublicKey {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        let (key_type, key_data) = match self {
            Self::ED25519(public_key) => (KeyType::ED25519, &public_key.0[..]),
            Self::SECP256K1(public_key) => (KeyType::SECP256K1, &public_key.0[..]),
            Self::MLDSA65(public_key) => (KeyType::MLDSA65, &public_key.0[..]),
        };
        write!(fmt, "{}:{}", key_type, bs58::encode(key_data).into_string())
    }
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(self, f)
    }
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
            Self::MLDSA65(public_key) => {
                BorshSerialize::serialize(&2u8, writer)?;
                writer.write_all(&*public_key.0)?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for PublicKey {
    fn deserialize_reader<R: Read>(rd: &mut R) -> std::io::Result<Self> {
        let key_type = KeyType::try_from(u8::deserialize_reader(rd)?)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;
        match key_type {
            KeyType::ED25519 => Ok(Self::ED25519(ED25519PublicKey(
                BorshDeserialize::deserialize_reader(rd)?,
            ))),
            KeyType::SECP256K1 => Ok(Self::SECP256K1(Secp256K1PublicKey(
                BorshDeserialize::deserialize_reader(rd)?,
            ))),
            KeyType::MLDSA65 => {
                // Read straight into the heap allocation to avoid a large stack temporary.
                let mut key = Box::new([0u8; ML_DSA_65_PUBLIC_KEY_LENGTH]);
                rd.read_exact(&mut *key)?;
                Ok(Self::MLDSA65(MlDsa65PublicKey(key)))
            }
        }
    }
}

impl serde::Serialize for PublicKey {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse()
            .map_err(|err: DataConversionError| serde::de::Error::custom(err.to_string()))
    }
}

impl FromStr for PublicKey {
    type Err = DataConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = split_key_type_data(value)?;
        Ok(match key_type {
            KeyType::ED25519 => Self::ED25519(ED25519PublicKey(
                bs58::decode(key_data).into_vec()?.try_into()?,
            )),
            KeyType::SECP256K1 => Self::SECP256K1(Secp256K1PublicKey(
                bs58::decode(key_data).into_vec()?.try_into()?,
            )),
            KeyType::MLDSA65 => Self::MLDSA65(MlDsa65PublicKey(
                bs58::decode(key_data).into_vec()?.try_into()?,
            )),
        })
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub struct Secp256K1PublicKey(pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

impl TryFrom<&[u8]> for Secp256K1PublicKey {
    type Error = DataConversionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(data.try_into()?))
    }
}

impl std::fmt::Debug for Secp256K1PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0).into_string(), f)
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub struct ED25519PublicKey(pub [u8; ed25519_dalek::PUBLIC_KEY_LENGTH]);

impl TryFrom<&[u8]> for ED25519PublicKey {
    type Error = DataConversionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(data.try_into()?))
    }
}

impl std::fmt::Debug for ED25519PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0).into_string(), f)
    }
}

/// FIPS 204 ML-DSA-65 public key.
///
/// The key bytes are heap-allocated so that the `PublicKey` enum stays small for the classical
/// curves.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct MlDsa65PublicKey(pub Box<[u8; ML_DSA_65_PUBLIC_KEY_LENGTH]>);

impl TryFrom<&[u8]> for MlDsa65PublicKey {
    type Error = DataConversionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let key: [u8; ML_DSA_65_PUBLIC_KEY_LENGTH] = data
            .try_into()
            .map_err(|_| Self::Error::IncorrectLength(data.len()))?;
        Ok(Self(Box::new(key)))
    }
}

impl std::fmt::Debug for MlDsa65PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0[..]).into_string(), f)
    }
}

#[cfg(test)]
impl bolero::TypeGenerator for MlDsa65PublicKey {
    fn generate<D: bolero::Driver>(driver: &mut D) -> Option<Self> {
        let mut key = Box::new([0u8; ML_DSA_65_PUBLIC_KEY_LENGTH]);
        for byte in key.iter_mut() {
            *byte = u8::generate(driver)?;
        }
        Some(Self(key))
    }
}
