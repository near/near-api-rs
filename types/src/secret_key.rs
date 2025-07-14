use ed25519_dalek::ed25519::signature::Signer;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use std::sync::LazyLock;

use crate::errors::{DataConversionError, InvalidFormatError, ParseKeyTypeError};
use crate::public_key::{ED25519PublicKey, Secp256K1PublicKey};
use crate::signature::{ED25519Signature, Secp256K1Signature};
use crate::{PublicKey, Signature};

pub static SECP256K1: LazyLock<secp256k1::Secp256k1<secp256k1::All>> =
    LazyLock::new(secp256k1::Secp256k1::new);

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum KeyType {
    ED25519 = 0,
    SECP256K1 = 1,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(match self {
            KeyType::ED25519 => "ed25519",
            KeyType::SECP256K1 => "secp256k1",
        })
    }
}

impl FromStr for KeyType {
    type Err = DataConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let lowercase_key_type = value.to_ascii_lowercase();
        match lowercase_key_type.as_str() {
            "ed25519" => Ok(KeyType::ED25519),
            "secp256k1" => Ok(KeyType::SECP256K1),
            _ => Err(DataConversionError::InvalidKeyFormat(
                InvalidFormatError::InvalidKeyFormat(lowercase_key_type.to_string()),
            )),
        }
    }
}

impl TryFrom<u8> for KeyType {
    type Error = ParseKeyTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(KeyType::ED25519),
            1 => Ok(KeyType::SECP256K1),
            unknown_key_type => Err(ParseKeyTypeError::UnknownKeyType(
                unknown_key_type.to_string(),
            )),
        }
    }
}

pub(crate) fn split_key_type_data(value: &str) -> Result<(KeyType, &str), DataConversionError> {
    if let Some((prefix, key_data)) = value.split_once(':') {
        Ok((KeyType::from_str(prefix)?, key_data))
    } else {
        // If there is no prefix then we Default to ED25519.
        Ok((KeyType::ED25519, value))
    }
}

#[derive(Clone, Eq)]
// This is actually a keypair, because ed25519_dalek api only has keypair.sign
// From ed25519_dalek doc: The first SECRET_KEY_LENGTH of bytes is the SecretKey
// The last PUBLIC_KEY_LENGTH of bytes is the public key, in total it's KEYPAIR_LENGTH
pub struct ED25519SecretKey(pub [u8; ed25519_dalek::KEYPAIR_LENGTH]);

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

/// Secret key container supporting different curves.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum SecretKey {
    ED25519(ED25519SecretKey),
    SECP256K1(secp256k1::SecretKey),
}

impl SecretKey {
    pub fn key_type(&self) -> KeyType {
        match self {
            SecretKey::ED25519(_) => KeyType::ED25519,
            SecretKey::SECP256K1(_) => KeyType::SECP256K1,
        }
    }

    #[cfg(feature = "rand")]
    pub fn from_random(key_type: KeyType) -> SecretKey {
        use secp256k1::rand::rngs::OsRng;

        match key_type {
            KeyType::ED25519 => {
                let keypair = ed25519_dalek::SigningKey::generate(&mut OsRng);
                SecretKey::ED25519(ED25519SecretKey(keypair.to_keypair_bytes()))
            }
            KeyType::SECP256K1 => SecretKey::SECP256K1(secp256k1::SecretKey::new(&mut OsRng)),
        }
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        match &self {
            SecretKey::ED25519(secret_key) => {
                let keypair = ed25519_dalek::SigningKey::from_keypair_bytes(&secret_key.0).unwrap();
                let signature = keypair.sign(data);
                Signature::ED25519(signature)
            }

            SecretKey::SECP256K1(secret_key) => {
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
            SecretKey::ED25519(secret_key) => PublicKey::ED25519(ED25519PublicKey(
                secret_key.0[ed25519_dalek::SECRET_KEY_LENGTH..]
                    .try_into()
                    .unwrap(),
            )),
            SecretKey::SECP256K1(secret_key) => {
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
            SecretKey::ED25519(key) => key,
            SecretKey::SECP256K1(_) => panic!(),
        }
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (key_type, key_data) = match self {
            SecretKey::ED25519(secret_key) => (KeyType::ED25519, &secret_key.0[..]),
            SecretKey::SECP256K1(secret_key) => (KeyType::SECP256K1, &secret_key[..]),
        };
        write!(f, "{}:{}", key_type, bs58::encode(key_data).into_string())
    }
}

impl FromStr for SecretKey {
    type Err = DataConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = split_key_type_data(s)?;
        Ok(match key_type {
            KeyType::ED25519 => Self::ED25519(ED25519SecretKey(
                bs58::decode(key_data).into_vec()?.try_into()?,
            )),
            KeyType::SECP256K1 => {
                let data = bs58::decode(key_data).into_vec()?;
                let sk = secp256k1::SecretKey::from_slice(&data)
                    .map_err(|err| DataConversionError::InvalidData(err.to_string()))?;
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

impl TryFrom<&[u8]> for Secp256K1Signature {
    type Error = DataConversionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(data.try_into().map_err(|_| {
            DataConversionError::IncorrectLength(data.len())
        })?))
    }
}

impl Debug for Secp256K1Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0).into_string(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyType, PublicKey, SecretKey, Signature};

    #[cfg(feature = "rand")]
    #[test]
    fn test_sign_verify() {
        for key_type in [KeyType::ED25519, KeyType::SECP256K1] {
            let secret_key = SecretKey::from_random(key_type);
            let public_key = secret_key.public_key();
            use sha2::Digest;
            let data = sha2::Sha256::digest(b"123").to_vec();
            let signature = secret_key.sign(&data);
            assert!(signature.verify(&data, &public_key));
        }
    }

    #[test]
    fn signature_verify_fuzzer() {
        bolero::check!().with_type().for_each(
            |(key_type, sign, data, public_key): &(KeyType, [u8; 65], Vec<u8>, PublicKey)| {
                let signature = match key_type {
                    KeyType::ED25519 => {
                        Signature::from_parts(KeyType::ED25519, &sign[..64]).unwrap()
                    }
                    KeyType::SECP256K1 => {
                        Signature::from_parts(KeyType::SECP256K1, &sign[..65]).unwrap()
                    }
                };
                let _ = signature.verify(&data, &public_key);
            },
        );
    }

    #[test]
    fn regression_signature_verification_originally_failed() {
        let signature = Signature::from_parts(KeyType::SECP256K1, &[4; 65]).unwrap();
        let _ = signature.verify(&[], &PublicKey::empty(KeyType::SECP256K1));
    }

    #[cfg(feature = "rand")]
    #[test]
    fn test_json_serialize_ed25519() {
        let sk = SecretKey::from_seed(KeyType::ED25519, "test");
        let pk = sk.public_key();
        let expected = "\"ed25519:DcA2MzgpJbrUATQLLceocVckhhAqrkingax4oJ9kZ847\"";
        assert_eq!(serde_json::to_string(&pk).unwrap(), expected);
        assert_eq!(pk, serde_json::from_str(expected).unwrap());
        assert_eq!(
            pk,
            serde_json::from_str("\"DcA2MzgpJbrUATQLLceocVckhhAqrkingax4oJ9kZ847\"").unwrap()
        );
        let pk2: PublicKey = pk.to_string().parse().unwrap();
        assert_eq!(pk, pk2);

        let expected = "\"ed25519:3KyUuch8pYP47krBq4DosFEVBMR5wDTMQ8AThzM8kAEcBQEpsPdYTZ2FPX5ZnSoLrerjwg66hwwJaW1wHzprd5k3\"";
        assert_eq!(serde_json::to_string(&sk).unwrap(), expected);
        assert_eq!(sk, serde_json::from_str(expected).unwrap());

        let signature = sk.sign(b"123");
        let expected = "\"ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj\"";
        assert_eq!(serde_json::to_string(&signature).unwrap(), expected);
        assert_eq!(signature, serde_json::from_str(expected).unwrap());
        let signature_str: String = signature.to_string();
        let signature2: Signature = signature_str.parse().unwrap();
        assert_eq!(signature, signature2);
    }

    #[cfg(feature = "rand")]
    #[test]
    fn test_json_serialize_secp256k1() {
        use sha2::Digest;
        let data = sha2::Sha256::digest(b"123").to_vec();

        let sk = SecretKey::from_seed(KeyType::SECP256K1, "test");
        let pk = sk.public_key();
        // cspell:disable-next-line
        let expected = "\"secp256k1:5ftgm7wYK5gtVqq1kxMGy7gSudkrfYCbpsjL6sH1nwx2oj5NR2JktohjzB6fbEhhRERQpiwJcpwnQjxtoX3GS3cQ\"";
        assert_eq!(serde_json::to_string(&pk).unwrap(), expected);
        assert_eq!(pk, serde_json::from_str(expected).unwrap());
        let pk2: PublicKey = pk.to_string().parse().unwrap();
        assert_eq!(pk, pk2);

        let expected = "\"secp256k1:X4ETFKtQkSGVoZEnkn7bZ3LyajJaK2b3eweXaKmynGx\"";
        assert_eq!(serde_json::to_string(&sk).unwrap(), expected);
        assert_eq!(sk, serde_json::from_str(expected).unwrap());

        let signature = sk.sign(&data);
        let expected = "\"secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6\"";
        assert_eq!(serde_json::to_string(&signature).unwrap(), expected);
        assert_eq!(signature, serde_json::from_str(expected).unwrap());
        let signature_str: String = signature.to_string();
        let signature2: Signature = signature_str.parse().unwrap();
        assert_eq!(signature, signature2);
    }

    #[cfg(feature = "rand")]
    #[test]
    fn test_borsh_serialization() {
        use borsh::BorshDeserialize;
        use sha2::Digest;

        let data = sha2::Sha256::digest(b"123").to_vec();
        for key_type in [KeyType::ED25519, KeyType::SECP256K1] {
            let sk = SecretKey::from_seed(key_type, "test");
            let pk = sk.public_key();
            let bytes = borsh::to_vec(&pk).unwrap();
            assert_eq!(PublicKey::try_from_slice(&bytes).unwrap(), pk);

            let signature = sk.sign(&data);
            let bytes = borsh::to_vec(&signature).unwrap();
            assert_eq!(Signature::try_from_slice(&bytes).unwrap(), signature);

            assert!(PublicKey::try_from_slice(&[0]).is_err());
            assert!(Signature::try_from_slice(&[0]).is_err());
        }
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
