use std::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    io::{Error, ErrorKind, Read, Write},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::Verifier;
use primitive_types::U256;
use secp256k1::Message;

use crate::{
    crypto::{
        public_key::Secp256K1PublicKey, secret_key::SECP256K1, split_key_type_data, KeyType,
        SECP256K1_SIGNATURE_LENGTH,
    },
    errors::{DataConversionError, SignatureErrors},
    PublicKey,
};

/// Signature container supporting different curves.
#[derive(Clone, PartialEq, Eq)]
pub enum Signature {
    ED25519(ed25519_dalek::Signature),
    SECP256K1(Secp256K1Signature),
}

// This `Hash` implementation is safe since it retains the property
// `k1 == k2 ⇒ hash(k1) == hash(k2)`.
impl Hash for Signature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::ED25519(sig) => sig.to_bytes().hash(state),
            Self::SECP256K1(sig) => sig.hash(state),
        };
    }
}

impl Signature {
    /// Construct Signature from key type and raw signature blob
    pub fn from_parts(
        signature_type: KeyType,
        signature_data: &[u8],
    ) -> Result<Self, DataConversionError> {
        match signature_type {
            KeyType::ED25519 => Ok(Self::ED25519(ed25519_dalek::Signature::from_bytes(
                <&[u8; ed25519_dalek::SIGNATURE_LENGTH]>::try_from(signature_data)?,
            ))),
            KeyType::SECP256K1 => Ok(Self::SECP256K1(Secp256K1Signature::try_from(
                signature_data,
            )?)),
        }
    }

    /// Verifies that this signature is indeed signs the data with given public key.
    /// Also if public key doesn't match on the curve returns `false`.
    pub fn verify(&self, data: &[u8], public_key: &PublicKey) -> bool {
        match (&self, public_key) {
            (Self::ED25519(signature), PublicKey::ED25519(public_key)) => {
                ed25519_dalek::VerifyingKey::from_bytes(&public_key.0)
                    .is_ok_and(|public_key| public_key.verify(data, signature).is_ok())
            }
            (Self::SECP256K1(signature), PublicKey::SECP256K1(public_key)) => {
                // cspell:ignore rsig pdata
                let rec_id =
                    match secp256k1::ecdsa::RecoveryId::from_i32(i32::from(signature.0[64])) {
                        Ok(r) => r,
                        Err(_) => return false,
                    };
                let rsig = match secp256k1::ecdsa::RecoverableSignature::from_compact(
                    &signature.0[0..64],
                    rec_id,
                ) {
                    Ok(r) => r,
                    Err(_) => return false,
                };
                let sig = rsig.to_standard();
                let pdata: [u8; 65] = {
                    // code borrowed from https://github.com/openethereum/openethereum/blob/98b7c07171cd320f32877dfa5aa528f585dc9a72/ethkey/src/signature.rs#L210
                    let mut temp = [4u8; 65];
                    temp[1..65].copy_from_slice(&public_key.0);
                    temp
                };
                let message = match secp256k1::Message::from_slice(data) {
                    Ok(m) => m,
                    Err(_) => return false,
                };
                let pub_key = match secp256k1::PublicKey::from_slice(&pdata) {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                SECP256K1.verify_ecdsa(&message, &sig, &pub_key).is_ok()
            }
            _ => false,
        }
    }

    pub const fn key_type(&self) -> KeyType {
        match self {
            Self::ED25519(_) => KeyType::ED25519,
            Self::SECP256K1(_) => KeyType::SECP256K1,
        }
    }
}

impl BorshSerialize for Signature {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::ED25519(signature) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&signature.to_bytes())?;
            }
            Self::SECP256K1(signature) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(&signature.0)?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for Signature {
    fn deserialize_reader<R: Read>(rd: &mut R) -> std::io::Result<Self> {
        let key_type = KeyType::try_from(u8::deserialize_reader(rd)?)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;
        match key_type {
            KeyType::ED25519 => {
                let array: [u8; ed25519_dalek::SIGNATURE_LENGTH] =
                    BorshDeserialize::deserialize_reader(rd)?;
                // Sanity-check that was performed by ed25519-dalek in from_bytes before version 2,
                // but was removed with version 2. It is not actually any good a check, but we have
                // it here in case we need to keep backward compatibility. Maybe this check is not
                // actually required, but please think carefully before removing it.
                if array[ed25519_dalek::SIGNATURE_LENGTH - 1] & 0b1110_0000 != 0 {
                    return Err(Error::new(ErrorKind::InvalidData, "signature error"));
                }
                Ok(Self::ED25519(ed25519_dalek::Signature::from_bytes(&array)))
            }
            KeyType::SECP256K1 => {
                let array: [u8; 65] = BorshDeserialize::deserialize_reader(rd)?;
                Ok(Self::SECP256K1(Secp256K1Signature(array)))
            }
        }
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let buf;
        let (key_type, key_data) = match self {
            Self::ED25519(signature) => {
                buf = signature.to_bytes();
                (KeyType::ED25519, &buf[..])
            }
            Self::SECP256K1(signature) => (KeyType::SECP256K1, &signature.0[..]),
        };
        write!(f, "{}:{}", key_type, bs58::encode(key_data).into_string())
    }
}

impl Debug for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(self, f)
    }
}

impl serde::Serialize for Signature {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Signature {
    type Err = DataConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (sig_type, sig_data) = split_key_type_data(value)?;
        Ok(match sig_type {
            KeyType::ED25519 => {
                let data = bs58::decode(sig_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?
                    .try_into()?;
                let sig = ed25519_dalek::Signature::from_bytes(&data);
                Self::ED25519(sig)
            }
            KeyType::SECP256K1 => Self::SECP256K1(Secp256K1Signature(
                bs58::decode(sig_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?
                    .try_into()?,
            )),
        })
    }
}

impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse()
            .map_err(|err: DataConversionError| serde::de::Error::custom(err.to_string()))
    }
}

const SECP256K1_N: U256 = U256([
    0xbfd25e8cd0364141,
    0xbaaedce6af48a03b,
    0xfffffffffffffffe,
    0xffffffffffffffff,
]);

// Half of SECP256K1_N + 1.
const SECP256K1_N_HALF_ONE: U256 = U256([
    0xdfe92f46681b20a1,
    0x5d576e7357a4501d,
    0xffffffffffffffff,
    0x7fffffffffffffff,
]);

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Secp256K1Signature(pub [u8; SECP256K1_SIGNATURE_LENGTH]);

impl Secp256K1Signature {
    pub fn check_signature_values(&self, reject_upper: bool) -> bool {
        let mut r_bytes = [0u8; 32];
        r_bytes.copy_from_slice(&self.0[0..32]);
        let r = U256::from(r_bytes);

        let mut s_bytes = [0u8; 32];
        s_bytes.copy_from_slice(&self.0[32..64]);
        let s = U256::from(s_bytes);

        let s_check = if reject_upper {
            // Reject upper range of s values (ECDSA malleability)
            SECP256K1_N_HALF_ONE
        } else {
            SECP256K1_N
        };

        r < SECP256K1_N && s < s_check
    }

    pub fn recover(&self, msg: [u8; 32]) -> Result<Secp256K1PublicKey, SignatureErrors> {
        let recoverable_sig = secp256k1::ecdsa::RecoverableSignature::from_compact(
            &self.0[0..64],
            secp256k1::ecdsa::RecoveryId::from_i32(i32::from(self.0[64])).map_err(|_| {
                SignatureErrors::InvalidSignatureData(secp256k1::Error::InvalidSignature)
            })?,
        )?;
        let msg = Message::from_slice(&msg).map_err(|_| {
            SignatureErrors::InvalidSignatureData(secp256k1::Error::InvalidSignature)
        })?;

        let res = SECP256K1
            .recover_ecdsa(&msg, &recoverable_sig)?
            .serialize_uncompressed();

        let pk = Secp256K1PublicKey::try_from(&res[1..65]).map_err(|_| {
            SignatureErrors::InvalidSignatureData(secp256k1::Error::InvalidSignature)
        })?;

        Ok(pk)
    }
}

impl TryFrom<&[u8]> for Secp256K1Signature {
    type Error = DataConversionError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(
            data.try_into()
                .map_err(|_| Self::Error::IncorrectLength(data.len()))?,
        ))
    }
}

impl Debug for Secp256K1Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0).into_string(), f)
    }
}
