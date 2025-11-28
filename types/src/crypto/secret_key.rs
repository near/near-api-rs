use std::{fmt::Display, str::FromStr, sync::LazyLock};

use ed25519_dalek::ed25519::signature::SignerMut;

use crate::{
    crypto::{
        public_key::{ED25519PublicKey, Secp256K1PublicKey},
        signature::Secp256K1Signature,
        split_key_type_data, KeyType,
    },
    errors::{DataConversionError, SecretKeyError},
    PublicKey, Signature,
};

pub static SECP256K1: LazyLock<secp256k1::Secp256k1<secp256k1::All>> =
    LazyLock::new(secp256k1::Secp256k1::new);

/// Secret key container supporting different curves.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum SecretKey {
    ED25519(ED25519SecretKey),
    SECP256K1(secp256k1::SecretKey),
}

impl SecretKey {
    pub const fn key_type(&self) -> KeyType {
        match self {
            Self::ED25519(_) => KeyType::ED25519,
            Self::SECP256K1(_) => KeyType::SECP256K1,
        }
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        match &self {
            Self::ED25519(secret_key) => {
                #[allow(clippy::expect_used)]
                let mut keypair = ed25519_dalek::SigningKey::from_keypair_bytes(&secret_key.0)
                    .expect("Invalid secret key");
                Signature::ED25519(keypair.sign(data))
            }

            Self::SECP256K1(secret_key) => {
                #[allow(clippy::expect_used)]
                let signature = SECP256K1.sign_ecdsa_recoverable(
                    &secp256k1::Message::from_slice(data).expect("32 bytes"),
                    secret_key,
                );
                let (rec_id, data) = signature.serialize_compact();
                let mut buf = [0; 65];
                buf[0..64].copy_from_slice(&data[0..64]);
                buf[64] = rec_id.to_i32() as u8;
                Signature::SECP256K1(Secp256K1Signature(buf))
            }
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match &self {
            Self::ED25519(secret_key) => {
                #[allow(clippy::expect_used)]
                let public_key = secret_key.0[ed25519_dalek::SECRET_KEY_LENGTH..]
                    .try_into()
                    .expect("Invalid secret keypair");
                PublicKey::ED25519(ED25519PublicKey(public_key))
            }
            Self::SECP256K1(secret_key) => {
                let pk = secp256k1::PublicKey::from_secret_key(&SECP256K1, secret_key);
                let serialized = pk.serialize_uncompressed();
                let mut public_key = Secp256K1PublicKey([0; 64]);
                public_key.0.copy_from_slice(&serialized[1..65]);
                PublicKey::SECP256K1(public_key)
            }
        }
    }

    pub fn unwrap_as_ed25519(&self) -> &ED25519SecretKey {
        match self {
            Self::ED25519(key) => key,
            Self::SECP256K1(_) => panic!("Secret key is not an ED25519 secret key"),
        }
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (key_type, key_data) = match self {
            Self::ED25519(secret_key) => (KeyType::ED25519, &secret_key.0[..]),
            Self::SECP256K1(secret_key) => (KeyType::SECP256K1, &secret_key[..]),
        };
        write!(f, "{}:{}", key_type, bs58::encode(key_data).into_string())
    }
}

impl FromStr for SecretKey {
    type Err = SecretKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = split_key_type_data(s)?;
        Ok(match key_type {
            KeyType::ED25519 => Self::ED25519(ED25519SecretKey(
                bs58::decode(key_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?
                    .try_into()?,
            )),
            KeyType::SECP256K1 => {
                let data = bs58::decode(key_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?;
                let sk = secp256k1::SecretKey::from_slice(&data)?;
                Self::SECP256K1(sk)
            }
        })
    }
}

impl serde::Serialize for SecretKey {
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

impl<'de> serde::Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::from_str(&s).map_err(|err| serde::de::Error::custom(err.to_string()))
    }
}

#[derive(Clone, Eq)]
// This is actually a keypair, because ed25519_dalek api only has keypair.sign
// From ed25519_dalek doc: The first SECRET_KEY_LENGTH of bytes is the SecretKey
// The last PUBLIC_KEY_LENGTH of bytes is the public key, in total it's KEYPAIR_LENGTH
pub struct ED25519SecretKey(pub [u8; ed25519_dalek::KEYPAIR_LENGTH]);

impl ED25519SecretKey {
    pub fn from_secret_key(secret_key: [u8; ed25519_dalek::SECRET_KEY_LENGTH]) -> Self {
        Self(ed25519_dalek::SigningKey::from_bytes(&secret_key).to_keypair_bytes())
    }
}

impl PartialEq for ED25519SecretKey {
    fn eq(&self, other: &Self) -> bool {
        self.0[..ed25519_dalek::SECRET_KEY_LENGTH] == other.0[..ed25519_dalek::SECRET_KEY_LENGTH]
    }
}

impl std::fmt::Debug for ED25519SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(
            &bs58::encode(&self.0[..ed25519_dalek::SECRET_KEY_LENGTH]).into_string(),
            f,
        )
    }
}
