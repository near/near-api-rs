#[derive(thiserror::Error, Debug)]
pub enum CryptoHashError {
    #[error(transparent)]
    Base58DecodeError(#[from] bs58::decode::Error),
    #[error("Incorrect hash length (expected 32, but {0} was given)")]
    IncorrectHashLength(usize),
}

#[derive(thiserror::Error, Debug)]
pub enum SignedDelegateActionError {
    #[error("Parsing of signed delegate action failed due to base64 sequence being invalid")]
    Base64DecodingError,
    #[error("Delegate action could not be deserialized from borsh: {0}")]
    BorshError(#[from] std::io::Error),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum DecimalNumberParsingError {
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
    #[error("Too long whole part: {0}")]
    LongWhole(String),
    #[error("Too long fractional part: {0}")]
    LongFractional(String),
}

#[derive(thiserror::Error, Debug)]
pub enum PublicKeyError {
    #[error("Invalid public key length: {0}")]
    InvalidLength(usize),
    #[error("Failed to decode base58: {0}")]
    Base58DecodeError(#[from] bs58::decode::Error),
    #[error("Invalid key format. Expected: [ed25519:..., secp256k1:...] but got: {0}")]
    InvalidKeyFormat(String),
    #[error("Invalid prefix. Expected: [ed25519, secp256k1] but got: {0}")]
    InvalidPrefix(String),
}

impl From<Vec<u8>> for PublicKeyError {
    fn from(value: Vec<u8>) -> Self {
        PublicKeyError::InvalidLength(value.len())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AccessKeyError {
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(#[from] PublicKeyError),
    #[error("Invalid access key: {0}")]
    InvalidAccessKey(#[from] std::num::ParseIntError),
}

#[derive(thiserror::Error, Debug)]
pub enum AccountViewError {
    #[error("Hash parsing error: {0}")]
    HashParsingError(#[from] CryptoHashError),
    #[error("Token parsing error: {0}")]
    TokenParsingError(#[from] std::num::ParseIntError),
}
