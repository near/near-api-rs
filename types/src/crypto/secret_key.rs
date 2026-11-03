use std::{fmt::Display, str::FromStr, sync::LazyLock};

use ed25519_dalek::ed25519::signature::SignerMut;
use ml_dsa::signature::Signer as MlDsaSigner;

use crate::{
    CryptoHash, PublicKey, Signature,
    crypto::{
        KeyType, ML_DSA_65_PUBLIC_KEY_LENGTH, ML_DSA_65_SECRET_KEY_LENGTH, ML_DSA_65_SEED_LENGTH,
        ML_DSA_65_SIGNATURE_LENGTH,
        public_key::{ED25519PublicKey, MlDsa65PublicKey, Secp256K1PublicKey},
        signature::{MlDsa65Signature, Secp256K1Signature},
        split_key_type_data,
    },
    errors::{DataConversionError, SecretKeyError},
};

pub static SECP256K1: LazyLock<secp256k1::Secp256k1<secp256k1::All>> =
    LazyLock::new(secp256k1::Secp256k1::new);

/// Secret key container supporting different curves.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum SecretKey {
    ED25519(ED25519SecretKey),
    SECP256K1(secp256k1::SecretKey),
    MLDSA65(MlDsa65SecretKey),
}

impl SecretKey {
    pub const fn key_type(&self) -> KeyType {
        match self {
            Self::ED25519(_) => KeyType::ED25519,
            Self::SECP256K1(_) => KeyType::SECP256K1,
            Self::MLDSA65(_) => KeyType::MLDSA65,
        }
    }

    pub fn sign(&self, data: CryptoHash) -> Signature {
        match &self {
            Self::ED25519(secret_key) => {
                let mut keypair = secret_key.0.clone();
                Signature::ED25519(keypair.sign(data.0.as_ref()))
            }

            Self::SECP256K1(secret_key) => {
                let signature = SECP256K1.sign_ecdsa_recoverable(&data.into(), secret_key);
                let (rec_id, data) = signature.serialize_compact();
                let mut buf = [0; 65];
                buf[0..64].copy_from_slice(&data[0..64]);
                buf[64] = rec_id.to_i32() as u8;
                Signature::SECP256K1(Secp256K1Signature(buf))
            }

            Self::MLDSA65(secret_key) => Signature::MLDSA65(secret_key.sign(&data.0)),
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match &self {
            Self::ED25519(secret_key) => {
                PublicKey::ED25519(ED25519PublicKey(secret_key.0.verifying_key().to_bytes()))
            }
            Self::SECP256K1(secret_key) => {
                let pk = secp256k1::PublicKey::from_secret_key(&SECP256K1, secret_key);
                let serialized = pk.serialize_uncompressed();
                let mut public_key = Secp256K1PublicKey([0; 64]);
                public_key.0.copy_from_slice(&serialized[1..65]);
                PublicKey::SECP256K1(public_key)
            }
            Self::MLDSA65(secret_key) => PublicKey::MLDSA65(secret_key.public_key()),
        }
    }

    pub fn unwrap_as_ed25519(&self) -> &ED25519SecretKey {
        match self {
            Self::ED25519(key) => key,
            Self::SECP256K1(_) | Self::MLDSA65(_) => {
                panic!("Secret key is not an ED25519 secret key")
            }
        }
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (key_type, key_data) = match self {
            Self::ED25519(secret_key) => (KeyType::ED25519, &secret_key.0.to_keypair_bytes()[..]),
            Self::SECP256K1(secret_key) => (KeyType::SECP256K1, &secret_key[..]),
            Self::MLDSA65(secret_key) => (KeyType::MLDSA65, &secret_key.0[..]),
        };
        write!(f, "{}:{}", key_type, bs58::encode(key_data).into_string())
    }
}

impl FromStr for SecretKey {
    type Err = SecretKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key_type, key_data) = split_key_type_data(s)?;
        Ok(match key_type {
            KeyType::ED25519 => {
                let data = bs58::decode(key_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?
                    .try_into()
                    .map_err(DataConversionError::from)?;

                let sk = ed25519_dalek::SigningKey::from_keypair_bytes(&data)?;
                Self::ED25519(ED25519SecretKey(sk))
            }
            KeyType::SECP256K1 => {
                let data = bs58::decode(key_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?;
                let sk = secp256k1::SecretKey::from_slice(&data)?;
                Self::SECP256K1(sk)
            }
            KeyType::MLDSA65 => {
                let data = bs58::decode(key_data)
                    .into_vec()
                    .map_err(DataConversionError::from)?;
                Self::MLDSA65(MlDsa65SecretKey::try_from(data.as_slice())?)
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
pub struct ED25519SecretKey(ed25519_dalek::SigningKey);

impl ED25519SecretKey {
    pub fn from_secret_key(secret_key: [u8; ed25519_dalek::SECRET_KEY_LENGTH]) -> Self {
        Self(ed25519_dalek::SigningKey::from_bytes(&secret_key))
    }
}

impl PartialEq for ED25519SecretKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_bytes() == other.0.as_bytes()
    }
}

impl std::fmt::Debug for ED25519SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        Display::fmt(&bs58::encode(&self.0.as_bytes()).into_string(), f)
    }
}

/// FIPS 204 ML-DSA-65 secret key.
///
/// This holds the *expanded* secret key (`skEncode`, 4032 bytes), which is the encoding NEAR
/// tooling uses for the `ml-dsa-65:` secret key string. A key can be created either from a
/// 32-byte seed ([`MlDsa65SecretKey::from_seed`]) or by parsing an expanded key
/// ([`TryFrom<&[u8]>`]).
///
/// The inner bytes are private and every constructor validates them, so a value of this type
/// is always a well-formed, internally consistent key. That is what makes
/// [`SecretKey::sign`] and [`SecretKey::public_key`] infallible.
#[derive(Clone, Eq, PartialEq)]
pub struct MlDsa65SecretKey(Box<[u8; ML_DSA_65_SECRET_KEY_LENGTH]>);

/// Offset of the packed `s1 || s2` secret vectors inside the expanded secret key:
/// they follow `rho` (32 bytes), `K` (32 bytes) and `tr` (64 bytes).
const ML_DSA_65_SK_S1_S2_OFFSET: usize = 32 + 32 + 64;
/// Length of the packed `s1 || s2` secret vectors inside the expanded secret key. ML-DSA-65 has
/// `l = 5` and `k = 6` polynomials of 256 coefficients, each coefficient packed in 4 bits.
const ML_DSA_65_SK_S1_S2_LENGTH: usize = (5 + 6) * 256 / 2;
/// Largest valid packed coefficient of `s1`/`s2`: coefficients live in `[-eta, eta]` with
/// `eta = 4` and are stored as `eta - coefficient`, so the packed nibble is in `0..=2 * eta`.
const ML_DSA_65_SK_MAX_PACKED_COEFFICIENT: u8 = 2 * 4;
/// Number of messages signed and verified to check that an imported secret key is consistent.
const ML_DSA_65_SELF_TEST_ROUNDS: u8 = 8;

impl MlDsa65SecretKey {
    /// Deterministically derives a secret key from a 32-byte FIPS 204 seed (`xi`).
    ///
    /// The seed must come from a cryptographically secure random source.
    #[allow(deprecated)] // `to_expanded` is the only way to get the NEAR key encoding.
    pub fn from_seed(seed: &[u8; ML_DSA_65_SEED_LENGTH]) -> Self {
        let signing_key =
            ml_dsa::ExpandedSigningKey::<ml_dsa::MlDsa65>::from_seed(&ml_dsa::Seed::from(*seed));
        let mut bytes = Box::new([0u8; ML_DSA_65_SECRET_KEY_LENGTH]);
        bytes.copy_from_slice(signing_key.to_expanded().as_slice());
        Self(bytes)
    }

    /// Rebuilds the `ml-dsa` signing key. Only called on bytes that were validated by a
    /// constructor, so this cannot panic.
    #[allow(deprecated)] // `from_expanded` is the only way to read the NEAR key encoding.
    fn signing_key(&self) -> ml_dsa::ExpandedSigningKey<ml_dsa::MlDsa65> {
        let bytes = ml_dsa::ExpandedSigningKeyBytes::<ml_dsa::MlDsa65>::from(*self.0);
        ml_dsa::ExpandedSigningKey::from_expanded(&bytes)
    }

    pub(crate) fn sign(&self, data: &[u8]) -> MlDsa65Signature {
        // Deterministic signing with an empty context string (pure ML-DSA), which is what NEAR
        // nodes verify. The trait method only fails for a context longer than 255 bytes.
        let signature = self.signing_key().sign(data);
        let mut bytes = Box::new([0u8; ML_DSA_65_SIGNATURE_LENGTH]);
        bytes.copy_from_slice(signature.encode().as_slice());
        MlDsa65Signature(bytes)
    }

    pub(crate) fn public_key(&self) -> MlDsa65PublicKey {
        let mut bytes = Box::new([0u8; ML_DSA_65_PUBLIC_KEY_LENGTH]);
        bytes.copy_from_slice(self.signing_key().verifying_key().encode().as_slice());
        MlDsa65PublicKey(bytes)
    }
}

impl TryFrom<&[u8]> for MlDsa65SecretKey {
    type Error = SecretKeyError;

    /// Parses an expanded (`skEncode`) ML-DSA-65 secret key.
    ///
    /// `ml-dsa` panics on a malformed expanded key, so the bytes are checked here first. The
    /// resulting key is then used to sign and verify several messages, which rejects keys whose
    /// parts (`s1`, `s2`, `t0`, `tr`) are individually well-formed but inconsistent.
    ///
    /// This is a sanity check, not a proof: `ml-dsa` cannot recompute `t0` from the other parts
    /// as nearcore's `aws-lc-rs` does, so a key with a tiny corruption in `s2`/`t0` can still be
    /// accepted (and then occasionally produce an invalid signature). Keys produced by NEAR
    /// tooling or by [`MlDsa65SecretKey::from_seed`] are always consistent.
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; ML_DSA_65_SECRET_KEY_LENGTH] = data
            .try_into()
            .map_err(|_| DataConversionError::IncorrectLength(data.len()))?;

        let packed_secret_vectors = &bytes
            [ML_DSA_65_SK_S1_S2_OFFSET..ML_DSA_65_SK_S1_S2_OFFSET + ML_DSA_65_SK_S1_S2_LENGTH];
        let has_out_of_range_coefficient = packed_secret_vectors.iter().any(|byte| {
            byte & 0x0f > ML_DSA_65_SK_MAX_PACKED_COEFFICIENT
                || byte >> 4 > ML_DSA_65_SK_MAX_PACKED_COEFFICIENT
        });
        if has_out_of_range_coefficient {
            return Err(SecretKeyError::InvalidMlDsa65SecretKey(
                "secret vector coefficient is out of range".to_owned(),
            ));
        }

        let secret_key = Self(Box::new(bytes));
        let signing_key = secret_key.signing_key();
        let verifying_key = signing_key.verifying_key();
        let is_consistent = (0..ML_DSA_65_SELF_TEST_ROUNDS).all(|round| {
            let message = [
                b"near-api-rs ml-dsa-65 secret key self test ".as_slice(),
                &[round],
            ]
            .concat();
            let signature = signing_key.sign(&message);
            verifying_key.verify_with_context(&message, &[], &signature)
        });
        if !is_consistent {
            return Err(SecretKeyError::InvalidMlDsa65SecretKey(
                "secret key is not internally consistent".to_owned(),
            ));
        }
        Ok(secret_key)
    }
}

impl std::fmt::Debug for MlDsa65SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // Unlike the other curves, never print secret key material.
        f.write_str("MlDsa65SecretKey(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const DEFAULT_ED25519_PRIVATE_KEY: &str = "ed25519:3tgdk2wPraJzT4nsTuf86UX41xgPNk3MHnq8epARMdBNs29AFEztAuaQ7iHddDfXG9F2RzV1XNQYgJyAyoW51UBB";
    pub const DEFAULT_ED25519_PUBLIC_KEY: &str =
        "ed25519:5BGSaf6YjVm7565VzWQHNxoyEjwr3jUpRJSGjREvU9dB";

    const DEFAULT_SECP256K1_PRIVATE_KEY: &str =
        "secp256k1:4rjSSUGmoSDEB4GQ7FybPdH9CSEuFNDfF85iK8CEF2fP";
    const DEFAULT_SECP256K1_PUBLIC_KEY: &str = "secp256k1:4JYvXJnrB3XUSNxJ3qY1ray24r9xz6mBF8Tuw3DxgXbwMKfmCJaRHiUzpqQZC6YkEbKnUZbMcBbwJwvfh7hYfyka";
    #[test]
    fn test_secret_key_display() {
        let secret_key = SecretKey::from_str(DEFAULT_ED25519_PRIVATE_KEY).unwrap();
        assert_eq!(secret_key.to_string(), DEFAULT_ED25519_PRIVATE_KEY);
    }

    #[test]
    fn test_secret_key_from_str() {
        let public_key = PublicKey::from_str(DEFAULT_ED25519_PUBLIC_KEY).unwrap();
        let secret_key = SecretKey::from_str(DEFAULT_ED25519_PRIVATE_KEY).unwrap();
        assert_eq!(public_key, secret_key.public_key());
    }

    #[test]
    fn test_secret_key_sign() {
        let secret_key = SecretKey::from_str(DEFAULT_ED25519_PRIVATE_KEY).unwrap();
        let data = CryptoHash::hash(b"hello");
        let signature = secret_key.sign(data);
        let public_key = secret_key.public_key();

        let verified = signature.verify(data, public_key);
        assert!(verified);
    }

    #[test]
    fn test_secp256k1_secret_key_display() {
        let secret_key = SecretKey::from_str(DEFAULT_SECP256K1_PRIVATE_KEY).unwrap();
        assert_eq!(secret_key.to_string(), DEFAULT_SECP256K1_PRIVATE_KEY);
    }

    #[test]
    fn test_secp256k1_secret_key_from_str() {
        let public_key = PublicKey::from_str(DEFAULT_SECP256K1_PUBLIC_KEY).unwrap();
        let secret_key = SecretKey::from_str(DEFAULT_SECP256K1_PRIVATE_KEY).unwrap();
        assert_eq!(public_key, secret_key.public_key());
    }

    #[test]
    fn test_secp256k1_secret_key_sign() {
        let secret_key = SecretKey::from_str(DEFAULT_SECP256K1_PRIVATE_KEY).unwrap();
        let data = CryptoHash::hash(b"hello");
        let signature = secret_key.sign(data);
        let public_key = secret_key.public_key();

        let verified = signature.verify(data, public_key);
        assert!(verified);
    }

    // ML-DSA-65 known-answer test. The seed, the digests of the derived keys and the signature
    // below were produced with `near-crypto` 0.37.4 (the aws-lc-rs based implementation used by
    // nearcore), so this guards both the key encoding and interoperability with the nodes.
    const ML_DSA_65_KAT_SEED: [u8; ML_DSA_65_SEED_LENGTH] = *b"near-api-rs ml-dsa-65 KAT       ";
    const ML_DSA_65_KAT_SECRET_KEY_SHA256: &str =
        "cb2e00666ef37dd53740de3810d01f6c35a3d953d1b90ca3ed358d06d93f81ab";
    const ML_DSA_65_KAT_PUBLIC_KEY_SHA256: &str =
        "e94bf039f413f8811cb53eff46e9bcfb293eee6c7d9b0a96a287ccf2428e0f24";
    // SHA-256 of the `ml-dsa-65:` signature produced by *this* implementation for the message
    // below. ML-DSA signing is deterministic here, so any change means the output drifted.
    const ML_DSA_65_KAT_OWN_SIGNATURE_SHA256: &str =
        "df72207cfe7d71e7162a49b70ef93469e47b090a39ca87cfa9b81359a5d24451";
    // Signature over `CryptoHash::hash(b"hello")` created by near-crypto.
    const ML_DSA_65_KAT_NEAR_CRYPTO_SIGNATURE: &str = "ml-dsa-65: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";

    fn sha256_hex(bytes: &[u8]) -> String {
        CryptoHash::hash(bytes)
            .0
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn ml_dsa_65_test_key() -> SecretKey {
        SecretKey::MLDSA65(MlDsa65SecretKey::from_seed(&[7; ML_DSA_65_SEED_LENGTH]))
    }

    #[test]
    fn test_ml_dsa_65_known_answer() {
        let secret_key = MlDsa65SecretKey::from_seed(&ML_DSA_65_KAT_SEED);
        assert_eq!(
            sha256_hex(&secret_key.0[..]),
            ML_DSA_65_KAT_SECRET_KEY_SHA256
        );
        let secret_key = SecretKey::MLDSA65(secret_key);
        let public_key = secret_key.public_key();
        assert_eq!(
            sha256_hex(public_key.key_data()),
            ML_DSA_65_KAT_PUBLIC_KEY_SHA256
        );

        let data = CryptoHash::hash(b"hello");

        // A signature created by nearcore's implementation must verify.
        let near_crypto_signature =
            Signature::from_str(ML_DSA_65_KAT_NEAR_CRYPTO_SIGNATURE).unwrap();
        assert!(near_crypto_signature.verify(data, public_key.clone()));

        // And our own signature is stable.
        let signature = secret_key.sign(data);
        assert!(signature.verify(data, public_key));
        assert_eq!(
            sha256_hex(&borsh::to_vec(&signature).unwrap()[1..]),
            ML_DSA_65_KAT_OWN_SIGNATURE_SHA256
        );
    }

    #[test]
    fn test_ml_dsa_65_secret_key_display_and_from_str() {
        let secret_key = ml_dsa_65_test_key();
        let serialized = secret_key.to_string();
        assert!(serialized.starts_with("ml-dsa-65:"));

        let parsed = SecretKey::from_str(&serialized).unwrap();
        assert_eq!(parsed, secret_key);
        assert_eq!(parsed.to_string(), serialized);
        assert!(matches!(parsed.key_type(), KeyType::MLDSA65));

        let public_key = PublicKey::from_str(&secret_key.public_key().to_string()).unwrap();
        assert_eq!(public_key, parsed.public_key());

        // Serde uses the same string form.
        let json = serde_json::to_string(&secret_key).unwrap();
        assert_eq!(
            serde_json::from_str::<SecretKey>(&json).unwrap(),
            secret_key
        );
    }

    #[test]
    fn test_ml_dsa_65_secret_key_sign() {
        let secret_key = ml_dsa_65_test_key();
        let public_key = secret_key.public_key();
        let data = CryptoHash::hash(b"hello");
        let signature = secret_key.sign(data);

        assert!(matches!(signature.key_type(), KeyType::MLDSA65));
        assert!(signature.verify(data, public_key.clone()));
        // Signing is deterministic.
        assert_eq!(signature, secret_key.sign(data));

        // Different data, a different key and a different curve all fail verification.
        assert!(!signature.verify(CryptoHash::hash(b"hello!"), public_key.clone()));
        let other_key =
            SecretKey::MLDSA65(MlDsa65SecretKey::from_seed(&[8; ML_DSA_65_SEED_LENGTH]));
        assert!(!signature.verify(data, other_key.public_key()));
        let ed25519_key = SecretKey::from_str(DEFAULT_ED25519_PRIVATE_KEY).unwrap();
        assert!(!signature.verify(data, ed25519_key.public_key()));

        // A flipped bit anywhere in the signature fails verification.
        let Signature::MLDSA65(mut tampered) = signature else {
            unreachable!("an ML-DSA-65 key produces an ML-DSA-65 signature");
        };
        tampered.0[0] ^= 1;
        assert!(!Signature::MLDSA65(tampered).verify(data, public_key));
    }

    #[test]
    fn test_ml_dsa_65_secret_key_debug_is_redacted() {
        let secret_key = ml_dsa_65_test_key();
        let debug = format!("{secret_key:?}");
        assert!(debug.contains("redacted"));
        assert!(!debug.contains(&secret_key.to_string()[10..30]));
    }

    #[test]
    fn test_ml_dsa_65_secret_key_rejects_malformed_input() {
        let valid = ml_dsa_65_test_key().to_string();
        let valid_bytes = bs58::decode(&valid["ml-dsa-65:".len()..])
            .into_vec()
            .unwrap();
        let encode = |bytes: &[u8]| format!("ml-dsa-65:{}", bs58::encode(bytes).into_string());

        // Wrong lengths.
        assert!(SecretKey::from_str(&encode(&valid_bytes[1..])).is_err());
        assert!(SecretKey::from_str(&encode(&[valid_bytes.as_slice(), &[0]].concat())).is_err());
        assert!(SecretKey::from_str(&encode(&[])).is_err());
        // Not base58.
        assert!(SecretKey::from_str("ml-dsa-65:0OIl").is_err());
        // The `ml-dsa-65-hash:` form nodes return in access key lists is not a key.
        assert!(SecretKey::from_str(&valid.replace("ml-dsa-65:", "ml-dsa-65-hash:")).is_err());

        // Packed `s1`/`s2` coefficients out of range: the underlying library would panic here.
        for fill in [0x00u8, 0xff, 0x88, 0x99, 0x9f] {
            assert!(
                SecretKey::from_str(&encode(&vec![fill; ML_DSA_65_SECRET_KEY_LENGTH])).is_err(),
                "fill {fill:#04x}"
            );
        }
        let mut out_of_range = valid_bytes.clone();
        out_of_range[ML_DSA_65_SK_S1_S2_OFFSET] = 0xff;
        assert!(matches!(
            SecretKey::from_str(&encode(&out_of_range)),
            Err(SecretKeyError::InvalidMlDsa65SecretKey(_))
        ));

        // Well-formed but inconsistent: a corrupted `rho` or `tr` no longer matches the rest.
        for index in [0, 32 + 32 + 10] {
            let mut inconsistent = valid_bytes.clone();
            inconsistent[index] ^= 0xff;
            assert!(
                matches!(
                    SecretKey::from_str(&encode(&inconsistent)),
                    Err(SecretKeyError::InvalidMlDsa65SecretKey(_))
                ),
                "byte {index}"
            );
        }
    }

    #[test]
    fn ml_dsa_65_secret_key_parsing_never_panics_fuzzer() {
        let SecretKey::MLDSA65(valid) = ml_dsa_65_test_key() else {
            unreachable!();
        };
        bolero::check!()
            .with_type::<Vec<(u16, u8)>>()
            .for_each(|mutations| {
                let mut bytes = *valid.0.clone();
                for (index, value) in mutations.iter().take(8) {
                    bytes[usize::from(*index) % ML_DSA_65_SECRET_KEY_LENGTH] = *value;
                }
                // Either verdict is fine; what matters is that parsing never panics and that
                // anything accepted can then be used to sign without panicking.
                if let Ok(key) = MlDsa65SecretKey::try_from(&bytes[..]) {
                    let key = SecretKey::MLDSA65(key);
                    let _ = key.sign(CryptoHash::hash(b"fuzz"));
                    let _ = key.public_key();
                }
            });
    }
}
