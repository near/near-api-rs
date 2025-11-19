use near_api_types::errors::DataConversionError;
use near_openapi_client::types::{
    FunctionCallError, InternalError, RpcQueryError, RpcRequestValidationErrorKind,
    RpcTransactionError,
};

#[derive(thiserror::Error, Debug)]
pub enum QueryCreationError {
    #[error("Staking pool factory account ID is not defined in the network config")]
    StakingPoolFactoryNotDefined,
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError<RpcError: std::fmt::Debug + Send + Sync> {
    #[error(transparent)]
    QueryCreationError(#[from] QueryCreationError),
    #[error("Unexpected response kind: expected {expected} type, but got {got:?}")]
    UnexpectedResponse {
        expected: &'static str,
        // Boxed to avoid large error type
        got: &'static str,
    },
    #[error("Failed to deserialize response: {0}")]
    DeserializeError(#[from] serde_json::Error),
    #[error("Query error: {0:?}")]
    QueryError(Box<RetryError<SendRequestError<RpcError>>>),
    #[error("Internal error: failed to get response. Please submit a bug ticket")]
    InternalErrorNoResponse,
    #[error("Failed to convert response: {0}")]
    ConversionError(Box<dyn std::error::Error + Send + Sync>),
}

impl<RpcError: std::fmt::Debug + Send + Sync> From<RetryError<SendRequestError<RpcError>>>
    for QueryError<RpcError>
{
    fn from(err: RetryError<SendRequestError<RpcError>>) -> Self {
        Self::QueryError(Box::new(err))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MetaSignError {
    #[error("Attempted to construct NonDelegateAction from Action::Delegate")]
    DelegateActionIsNotSupported,

    #[error(transparent)]
    SignerError(#[from] SignerError),
}

#[derive(thiserror::Error, Debug)]
pub enum SignerError {
    #[error("Public key is not available")]
    PublicKeyIsNotAvailable,
    #[error("Secret key is not available")]
    SecretKeyIsNotAvailable,
    #[error("Failed to fetch nonce: {0:?}")]
    FetchNonceError(Box<QueryError<RpcQueryError>>),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[cfg(feature = "ledger")]
    #[error(transparent)]
    LedgerError(#[from] LedgerError),
}

#[derive(thiserror::Error, Debug)]
pub enum SecretError {
    #[error("Failed to process seed phrase: {0}")]
    BIP39Error(#[from] bip39::Error),
    #[error("Failed to derive key from seed phrase: Invalid Index")]
    DeriveKeyInvalidIndex,
}

#[derive(thiserror::Error, Debug)]
pub enum AccessKeyFileError {
    #[error("Failed to read access key file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse access key file: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error(transparent)]
    SecretError(#[from] SecretError),
    #[error("Public key is not linked to the private key")]
    PrivatePublicKeyMismatch,
}

#[cfg(feature = "keystore")]
#[derive(thiserror::Error, Debug)]
pub enum KeyStoreError {
    #[error(transparent)]
    Keystore(#[from] keyring::Error),
    #[error("Failed to query account keys: {0:?}")]
    QueryError(QueryError<RpcQueryError>),
    #[error("Failed to parse access key file: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error(transparent)]
    SecretError(#[from] SecretError),
    #[error("Task execution error: {0}")]
    TaskExecutionError(#[from] tokio::task::JoinError),
}

#[cfg(feature = "ledger")]
#[derive(thiserror::Error, Debug)]
pub enum LedgerError {
    #[error(
        "Buffer overflow on Ledger device occurred. \
Transaction is too large for signature. \
This is resolved in https://github.com/dj8yfo/app-near-rs . \
The status is tracked in `About` section."
    )]
    BufferOverflow,
    #[error("Ledger device error: {0:?}")]
    LedgerError(near_ledger::NEARLedgerError),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Task execution error: {0}")]
    TaskExecutionError(#[from] tokio::task::JoinError),
    #[error("Signature is not expected to fail on deserialization: {0}")]
    SignatureDeserializationError(String),
    #[error("Failed to cache public key: {0}")]
    SetPublicKeyError(#[from] tokio::sync::SetError<crate::PublicKey>),
}

#[cfg(feature = "ledger")]
impl From<near_ledger::NEARLedgerError> for LedgerError {
    fn from(err: near_ledger::NEARLedgerError) -> Self {
        const SW_BUFFER_OVERFLOW: &str = "0x6990";

        match err {
            near_ledger::NEARLedgerError::APDUExchangeError(msg)
                if msg.contains(SW_BUFFER_OVERFLOW) =>
            {
                Self::BufferOverflow
            }
            near_ledger_error => Self::LedgerError(near_ledger_error),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SecretBuilderError<E: std::fmt::Debug> {
    #[error("Public key is not available")]
    PublicKeyIsNotAvailable,
    #[error(transparent)]
    SecretError(#[from] SecretError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    CallbackError(E),
}

#[derive(thiserror::Error, Debug)]
pub enum BuilderError {
    #[error("Incorrect arguments: {0}")]
    IncorrectArguments(#[from] serde_json::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum AccountCreationError {
    #[error(transparent)]
    BuilderError(#[from] BuilderError),

    #[error(transparent)]
    PublicKeyParsingError(#[from] PublicKeyParsingError),

    #[error("Top-level account is not allowed")]
    TopLevelAccountIsNotAllowed,

    #[error("Linkdrop is not defined in the network config")]
    LinkdropIsNotDefined,

    #[error("Account should be created as a sub-account of the signer or linkdrop account")]
    AccountShouldBeSubAccountOfSignerOrLinkdrop,
}

#[derive(thiserror::Error, Debug)]
pub enum FaucetError {
    #[error(
        "The <{0}> network config does not have a defined faucet (helper service) that can sponsor the creation of an account."
    )]
    FaucetIsNotDefined(String),
    #[error("Failed to send message: {0}")]
    SendError(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum RetryError<E> {
    #[error("No RPC endpoints are defined in the network config")]
    NoRpcEndpoints,
    #[error("Invalid API key: {0}")]
    InvalidApiKey(#[from] reqwest::header::InvalidHeaderValue),
    #[error("Request failed. Retries exhausted. Last error: {0}")]
    RetriesExhausted(E),
    #[error("Critical error: {0}")]
    Critical(E),
}

#[derive(thiserror::Error, Debug)]
pub enum ExecuteTransactionError {
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Transaction signing error: {0}")]
    SignerError(#[from] SignerError),
    #[error("Meta-signing error: {0}")]
    MetaSignError(#[from] MetaSignError),
    #[error("Pre-query error: {0:?}")]
    PreQueryError(QueryError<RpcQueryError>),
    #[error("Transaction error: {0:?}")]
    TransactionError(RetryError<SendRequestError<RpcTransactionError>>),
    #[error(transparent)]
    NonEmptyVecError(#[from] NonEmptyVecError),
    #[error("Data conversion error: {0}")]
    DataConversionError(#[from] DataConversionError),
}

#[derive(thiserror::Error, Debug)]
pub enum ExecuteMetaTransactionsError {
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Meta-signing error: {0}")]
    SignError(#[from] MetaSignError),
    #[error("Pre-query error: {0:?}")]
    PreQueryError(QueryError<RpcQueryError>),

    #[error("Relayer is not defined in the network config")]
    RelayerIsNotDefined,

    #[error("Failed to send meta-transaction: {0}")]
    SendError(#[from] reqwest::Error),

    #[error(transparent)]
    NonEmptyVecError(#[from] NonEmptyVecError),
}

#[derive(thiserror::Error, Debug)]
pub enum FTValidatorError {
    #[deprecated(
        since = "0.7.3",
        note = "this error is unused as we are not falling if no metadata provided"
    )]
    #[error("Metadata is not provided")]
    NoMetadata,
    #[error("Decimals mismatch: expected {expected}, got {got}")]
    DecimalsMismatch { expected: u8, got: u8 },
    #[error("Storage deposit is needed")]
    StorageDepositNeeded,
}

#[derive(thiserror::Error, Debug)]
pub enum FastNearError {
    #[error("FastNear URL is not defined in the network config")]
    FastNearUrlIsNotDefined,
    #[error("Failed to send request: {0}")]
    SendError(#[from] reqwest::Error),
    #[error("Url parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

//TODO: it's better to have a separate errors, but for now it would be aggregated here
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Query error: {0:?}")]
    QueryError(QueryError<RpcQueryError>),

    #[error("Query creation error: {0}")]
    RequestBuilderError(#[from] BuilderError),

    #[error("FT Validation Error: {0}")]
    FTValidatorError(#[from] FTValidatorError),

    #[error("Account creation error: {0}")]
    AccountCreationError(#[from] AccountCreationError),
}

#[derive(thiserror::Error, Debug)]
pub enum MultiTransactionError {
    #[error(transparent)]
    NonEmptyVecError(#[from] NonEmptyVecError),

    #[error(transparent)]
    SignerError(#[from] SignerError),
    #[error("Duplicate signer")]
    DuplicateSigner,

    #[error(transparent)]
    SignedTransactionError(#[from] ExecuteTransactionError),

    #[error("Failed to send meta-transaction: {0}")]
    MetaTransactionError(#[from] ExecuteMetaTransactionsError),
}

#[derive(thiserror::Error, Debug)]
pub enum NonEmptyVecError {
    #[error("Vector is empty")]
    EmptyVector,
}

#[derive(thiserror::Error, Debug)]
pub enum PublicKeyParsingError {
    #[error("Invalid prefix: {0}")]
    InvalidPrefix(String),
    #[error("Base64 decoding error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Invalid Key Length")]
    InvalidKeyLength,
}

impl From<Vec<u8>> for PublicKeyParsingError {
    fn from(_: Vec<u8>) -> Self {
        Self::InvalidKeyLength
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SendRequestError<RpcError: std::fmt::Debug + Send + Sync> {
    #[error("Query creation error: {0}")]
    RequestCreationError(#[from] QueryCreationError),
    #[error("Transport error: {0}")]
    TransportError(near_openapi_client::Error<()>),
    #[error("Wasm execution failed with error: {0}")]
    WasmExecutionError(#[from] FunctionCallError),
    #[error("Internal error: {0:?}")]
    InternalError(#[from] InternalError),
    #[error("Request validation error: {0:?}")]
    RequestValidationError(#[from] RpcRequestValidationErrorKind),
    #[error("Server error: {0}")]
    ServerError(RpcError),
}

/// That's a BIG BIG HACK to handle inconsistent RPC errors
///
/// Node responds as a message instead of an error object, so we need to parse the message and return the error.
/// https://github.com/near/nearcore/blob/ae6fd841eaad76a090a02e9dcf7406bc79b81dbb/chain/jsonrpc/src/lib.rs#L204
///
/// TODO: remove this once we have a proper error handling in the RPC API.
/// - https://github.com/near/near-sdk-rs/pull/1165
/// - nearcore PR
impl<RpcError: std::fmt::Debug + Send + Sync> From<near_openapi_client::Error<()>>
    for SendRequestError<RpcError>
{
    fn from(err: near_openapi_client::Error<()>) -> Self {
        if let near_openapi_client::Error::InvalidResponsePayload(bytes, _error) = &err {
            let error = serde_json::from_slice::<serde_json::Value>(bytes)
                .unwrap_or_default()
                .get("result")
                .and_then(|result| result.get("error"))
                .and_then(|message| message.as_str())
                .and_then(|message| message.strip_prefix("wasm execution failed with error: "))
                .and_then(|message| serde_dbgfmt::from_str::<FunctionCallError>(message).ok());
            if let Some(error) = error {
                return Self::WasmExecutionError(error);
            }
        }

        Self::TransportError(err)
    }
}
