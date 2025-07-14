use near_openapi_types::TxExecutionError;

use crate::transaction_result::ExecutionFailure;

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
pub enum InvalidFormatError {
    #[error("Invalid key format. Expected: [ed25519, secp256k1] but got: {0}")]
    InvalidKeyFormat(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ParseKeyTypeError {
    #[error("Unknown key type: {0}")]
    UnknownKeyType(String),
}

#[derive(thiserror::Error, Debug)]
pub enum DataConversionError {
    #[error("Base64 decoding error: {0}")]
    Base64DecodingError(#[from] base64::DecodeError),
    #[error("Base58 decoding error: {0}")]
    Base58DecodingError(#[from] bs58::decode::Error),
    #[error("Borsh deserialization error: {0}")]
    BorshDeserializationError(#[from] borsh::io::Error),
    #[error("JSON deserialization error: {0}")]
    JsonDeserializationError(#[from] serde_json::Error),
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Incorrect length: {0}")]
    IncorrectLength(usize),
    #[error("Invalid public key: {0}")]
    InvalidKeyFormat(#[from] InvalidFormatError),
    #[error("Delegate action is not supported")]
    DelegateActionNotSupported,
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl From<Vec<u8>> for DataConversionError {
    fn from(value: Vec<u8>) -> Self {
        Self::IncorrectLength(value.len())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Data conversion error: {0}")]
    DataConversionError(#[from] DataConversionError),
    #[error("Execution failure: {0:?}")]
    TransactionFailure(Box<ExecutionFailure>),
    #[error("EOF while parsing a value at line 1 column 0")]
    EofWhileParsingValue,
    #[error("Executing transaction failed")]
    TransactionExecutionFailed(Box<TxExecutionError>),
    #[error("Execution pending or unknown")]
    ExecutionPendingOrUnknown,
}

impl From<ExecutionFailure> for ExecutionError {
    fn from(value: ExecutionFailure) -> Self {
        Self::TransactionFailure(Box::new(value))
    }
}

impl From<TxExecutionError> for ExecutionError {
    fn from(value: TxExecutionError) -> Self {
        Self::TransactionExecutionFailed(Box::new(value))
    }
}
