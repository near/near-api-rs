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
