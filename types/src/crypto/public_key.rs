use std::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    io::{Error, ErrorKind, Read, Write},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    crypto::{KeyType, SECP256K1_PUBLIC_KEY_LENGTH, split_key_type_data},
    errors::DataConversionError,
};

/// Public key container supporting different curves.
#[derive(Clone, PartialEq, PartialOrd, Ord, Eq)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
}

impl PublicKey {
    // `is_empty` always returns false, so there is no point in adding it
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        const ED25519_LEN: usize = ed25519_dalek::PUBLIC_KEY_LENGTH + 1;
        match self {
            Self::ED25519(_) => ED25519_LEN,
            Self::SECP256K1(_) => 65,
        }
    }

    pub fn empty(key_type: KeyType) -> Self {
        match key_type {
            KeyType::ED25519 => {
                PublicKey::ED25519(ED25519PublicKey([0u8; ed25519_dalek::PUBLIC_KEY_LENGTH]))
            }
            KeyType::SECP256K1 => PublicKey::SECP256K1(Secp256K1PublicKey([0u8; 64])),
        }
    }

    pub fn key_type(&self) -> KeyType {
        match self {
            Self::ED25519(_) => KeyType::ED25519,
            Self::SECP256K1(_) => KeyType::SECP256K1,
        }
    }

    pub fn key_data(&self) -> &[u8] {
        match self {
            Self::ED25519(key) => &key.0,
            Self::SECP256K1(key) => &key.0,
        }
    }

    pub fn unwrap_as_ed25519(&self) -> &ED25519PublicKey {
        match self {
            Self::ED25519(key) => key,
            Self::SECP256K1(_) => panic!(),
        }
    }

    pub fn unwrap_as_secp256k1(&self) -> &Secp256K1PublicKey {
        match self {
            Self::SECP256K1(key) => key,
            Self::ED25519(_) => panic!(),
        }
    }
}

impl TryFrom<near_openapi_types::PublicKey> for PublicKey {
    type Error = DataConversionError;
    fn try_from(val: near_openapi_types::PublicKey) -> Result<Self, Self::Error> {
        PublicKey::from_str(&val.0)
    }
}

impl From<PublicKey> for near_openapi_types::PublicKey {
    fn from(val: PublicKey) -> Self {
        near_openapi_types::PublicKey(val.to_string())
    }
}

// This `Hash` implementation is safe since it retains the property
// `k1 == k2 ⇒ hash(k1) == hash(k2)`.
impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PublicKey::ED25519(public_key) => {
                state.write_u8(0u8);
                state.write(&public_key.0);
            }
            PublicKey::SECP256K1(public_key) => {
                state.write_u8(1u8);
                state.write(&public_key.0);
            }
        }
    }
}

impl Display for PublicKey {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        let (key_type, key_data) = match self {
            PublicKey::ED25519(public_key) => (KeyType::ED25519, &public_key.0[..]),
            PublicKey::SECP256K1(public_key) => (KeyType::SECP256K1, &public_key.0[..]),
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
            PublicKey::ED25519(public_key) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
            PublicKey::SECP256K1(public_key) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(&public_key.0)?;
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
            KeyType::ED25519 => Ok(PublicKey::ED25519(ED25519PublicKey(
                BorshDeserialize::deserialize_reader(rd)?,
            ))),
            KeyType::SECP256K1 => Ok(PublicKey::SECP256K1(Secp256K1PublicKey(
                BorshDeserialize::deserialize_reader(rd)?,
            ))),
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
        })
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
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
