use std::str::FromStr;

use omni_transaction::near::types::ED25519PublicKey;

use crate::{Convert, PublicKey};

impl From<Convert<PublicKey>> for String {
    fn from(convert: Convert<PublicKey>) -> Self {
        match convert.0 {
            PublicKey::ED25519(key) => format!("ed25519:{}", bs58::encode(key.0).into_string()),
            PublicKey::SECP256K1(key) => format!("secp256k1:{}", bs58::encode(key.0).into_string()),
        }
    }
}

impl From<Convert<String>> for PublicKey {
    fn from(convert: Convert<String>) -> Self {
        Convert(near_openapi_types::PublicKey(convert.0)).into()
    }
}

impl From<Convert<PublicKey>> for near_openapi_types::PublicKey {
    fn from(convert: Convert<PublicKey>) -> Self {
        near_openapi_types::PublicKey(convert.into())
    }
}

impl From<Convert<PublicKey>> for near_crypto::PublicKey {
    fn from(convert: Convert<PublicKey>) -> Self {
        let string: String = convert.into();
        near_crypto::PublicKey::from_str(&string).unwrap()
    }
}

impl From<Convert<near_crypto::PublicKey>> for PublicKey {
    fn from(convert: Convert<near_crypto::PublicKey>) -> Self {
        let string: String = convert.0.to_string();
        Convert(near_openapi_types::PublicKey(string)).into()
    }
}

impl From<Convert<near_openapi_types::PublicKey>> for PublicKey {
    fn from(convert: Convert<near_openapi_types::PublicKey>) -> Self {
        let mut convert = convert.0.split(':');
        match convert.next() {
            Some("ed25519") => {
                let key = convert.next().unwrap();
                let key = bs58::decode(key).into_vec().unwrap();
                PublicKey::ED25519(ED25519PublicKey(key.try_into().unwrap()))
            }
            Some("secp256k1") => {
                let key = convert.next().unwrap();
                let key = bs58::decode(key).into_vec().unwrap();
                PublicKey::SECP256K1(omni_transaction::near::types::Secp256K1PublicKey(
                    key.try_into().unwrap(),
                ))
            }
            _ => panic!("Invalid public key"),
        }
    }
}
