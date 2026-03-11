use std::sync::Arc;

use crate::rpc_client::RpcCallError;
use near_openrpc_client::RpcError;
use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum QueryCreationError {
    #[error("Staking pool factory account ID is not defined in the network config")]
    StakingPoolFactoryNotDefined,
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error(transparent)]
    QueryCreationError(#[from] QueryCreationError),
    #[error("Failed to deserialize response: {0}")]
    DeserializeError(#[from] serde_json::Error),
    #[error("Query error: {0:?}")]
    QueryError(Box<RetryError<SendRequestError>>),
    #[error("Internal error: failed to get response. Please submit a bug ticket")]
    InternalErrorNoResponse,
    #[error("Argument serialization error: {0}")]
    ArgumentValidationError(#[from] ArgumentValidationError),
    #[error("Failed to convert response: {0}")]
    ConversionError(Box<dyn std::error::Error + Send + Sync>),
}

impl From<RetryError<SendRequestError>> for QueryError {
    fn from(err: RetryError<SendRequestError>) -> Self {
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
pub enum PublicKeyError {
    #[error("Public key is not available")]
    PublicKeyIsNotAvailable,
    #[cfg(feature = "ledger")]
    #[error("Failed to cache public key: {0}")]
    SetPublicKeyError(#[from] tokio::sync::SetError<crate::PublicKey>),
}

#[derive(thiserror::Error, Debug)]
pub enum SignerError {
    #[error(transparent)]
    PublicKeyError(#[from] PublicKeyError),
    #[error("Secret key is not available")]
    SecretKeyIsNotAvailable,
    #[error("Failed to fetch nonce: {0:?}")]
    FetchNonceError(Box<QueryError>),
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
    #[error(transparent)]
    PublicKeyError(#[from] PublicKeyError),
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
    #[error(transparent)]
    PublicKeyError(#[from] PublicKeyError),
}

#[cfg(feature = "keystore")]
#[derive(thiserror::Error, Debug)]
pub enum KeyStoreError {
    #[error(transparent)]
    Keystore(#[from] keyring::Error),
    #[error("Failed to query account keys: {0:?}")]
    QueryError(QueryError),
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

#[derive(thiserror::Error, Debug, Clone)]
pub enum ArgumentValidationError {
    #[error("Failed to serialize arguments as JSON: {0}")]
    JsonSerializationError(Arc<serde_json::Error>),
    #[error("Failed to serialize arguments as Borsh: {0}")]
    BorshSerializationError(Arc<std::io::Error>),
    #[error("Account creation error: {0}")]
    AccountCreationError(#[from] AccountCreationError),
    #[error("Multiple errors: {0:?}")]
    MultipleErrors(Vec<Self>),
}

impl ArgumentValidationError {
    pub const fn multiple(errors: Vec<Self>) -> Self {
        Self::MultipleErrors(errors)
    }
}

impl From<serde_json::Error> for ArgumentValidationError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonSerializationError(Arc::new(err))
    }
}

impl From<std::io::Error> for ArgumentValidationError {
    fn from(err: std::io::Error) -> Self {
        Self::BorshSerializationError(Arc::new(err))
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum AccountCreationError {
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
    #[error(transparent)]
    ArgumentValidationError(#[from] ArgumentValidationError),

    #[error("Pre-query error: {0:?}")]
    PreQueryError(QueryError),
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Meta-signing error: {0}")]
    MetaSignError(#[from] MetaSignError),
    #[error("Transaction signing error: {0}")]
    SignerError(#[from] SignerError),

    #[error("Transaction error: {0:?}")]
    TransactionError(RetryError<SendRequestError>),
    #[error("Data conversion error: {0}")]
    DataConversionError(#[from] near_api_types::errors::DataConversionError),
}

#[derive(thiserror::Error, Debug)]
pub enum ExecuteMetaTransactionsError {
    #[error(transparent)]
    ArgumentValidationError(#[from] ArgumentValidationError),

    #[error("Pre-query error: {0:?}")]
    PreQueryError(QueryError),
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Relayer is not defined in the network config")]
    RelayerIsNotDefined,

    #[error("Meta-signing error: {0}")]
    SignError(#[from] MetaSignError),

    #[error("Failed to send meta-transaction: {0}")]
    SendError(#[from] reqwest::Error),
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

//TODO: it's better to have a separate errors, but for now it would be aggregated here
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Query error: {0:?}")]
    QueryError(QueryError),

    #[error(transparent)]
    ArgumentValidationError(#[from] ArgumentValidationError),

    #[error("FT Validation Error: {0}")]
    FTValidatorError(#[from] FTValidatorError),

    #[error("Account creation error: {0}")]
    AccountCreationError(#[from] AccountCreationError),
}

#[derive(thiserror::Error, Debug)]
pub enum SendRequestError {
    #[error("Query creation error: {0}")]
    RequestCreationError(#[from] QueryCreationError),
    #[error("Transport error: {0}")]
    TransportError(RpcCallError),
    #[error("Server error: {0}")]
    ServerError(RpcError),
    #[error("Contract execution error: {0}")]
    ContractExecutionError(String),
}

impl SendRequestError {
    /// Returns the underlying [`RpcError`] if this is a server error.
    pub const fn as_rpc_error(&self) -> Option<&RpcError> {
        match self {
            Self::ServerError(e) => Some(e),
            _ => None,
        }
    }

    /// Attempts to deserialize the RPC error cause into a typed per-method error enum.
    ///
    /// Convenience wrapper around [`RpcError::try_cause_as`]. Returns `None` if
    /// this isn't a server error or if the server error has no cause.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use near_openrpc_client::errors::RpcQueryError;
    ///
    /// match send_err.try_cause_as::<RpcQueryError>() {
    ///     Some(Ok(RpcQueryError::UnknownAccount { requested_account_id, .. })) => {
    ///         println!("Account {requested_account_id} not found");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    pub fn try_cause_as<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Option<Result<T, serde_json::Error>> {
        self.as_rpc_error()?.try_cause_as()
    }
}

impl From<RpcCallError> for SendRequestError {
    fn from(err: RpcCallError) -> Self {
        match err {
            RpcCallError::Rpc(rpc_err) => Self::ServerError(rpc_err),
            other => Self::TransportError(other),
        }
    }
}
