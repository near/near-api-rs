use near_jsonrpc_client::{
    errors::JsonRpcError,
    methods::{query::RpcQueryRequest, tx::RpcTransactionError, RpcMethod},
};
use near_jsonrpc_primitives::types::query::QueryResponseKind;

#[derive(thiserror::Error, Debug)]
pub enum QueryCreationError {
    #[error("Staking pool factory account ID is not defined in the network config")]
    StakingPoolFactoryNotDefined,
}

#[derive(thiserror::Error, Debug)]
pub enum QueryError<Method: RpcMethod>
where
    Method::Error: std::fmt::Debug + std::fmt::Display,
{
    #[error(transparent)]
    QueryCreationError(#[from] QueryCreationError),
    #[error("Unexpected response kind: expected {expected} type, but got {got:?}")]
    UnexpectedResponse {
        expected: &'static str,
        got: QueryResponseKind,
    },
    #[error("Failed to deserialize response: {0}")]
    DeserializeError(#[from] serde_json::Error),
    #[error(transparent)]
    JsonRpcError(#[from] JsonRpcError<Method::Error>),
    #[error("Internal error: failed to get response. Please submit a bug ticket")]
    InternalErrorNoResponse,
}

#[derive(thiserror::Error, Debug)]
pub enum MetaSignError {
    // near_primitives::action::delegate::IsDelegateAction is private, so we redefined it here
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
    #[error("Failed to fetch nonce: {0}")]
    FetchNonceError(#[from] QueryError<RpcQueryRequest>),

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
}
#[derive(thiserror::Error, Debug)]
pub enum KeyStoreError {
    #[error(transparent)]
    Keystore(#[from] keyring::Error),
    #[error("Failed to query account keys: {0}")]
    QueryError(#[from] QueryError<RpcQueryRequest>),
    #[error("Failed to parse access key file: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error(transparent)]
    SecretError(#[from] SecretError),
}

#[derive(thiserror::Error, Debug)]
pub enum LedgerError {
    #[error(
        "Buffer overflow on Ledger device occured. \
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
    SignatureDeserializationError(#[from] near_crypto::ParseSignatureError),
}

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
pub enum SecretBuilderkError<E: std::fmt::Debug + std::fmt::Display> {
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

    #[error("Top-level account is not allowed")]
    TopLevelAccountIsNotAllowed,

    #[error("Linkdrop is not defined in the network config")]
    LinkdropIsNotDefined,

    #[error("Account should be created as a subaccount of the signer or linkdrop account")]
    AccountShouldBeSubaccountOfSignerOrLinkdrop,
}

#[derive(thiserror::Error, Debug)]
pub enum FaucetError {
    #[error("The <{0}> network config does not have a defined faucet (helper service) that can sponsor the creation of an account.")]
    FaucetIsNotDefined(String),
    #[error("Failed to send message: {0}")]
    SendError(#[from] reqwest::Error),
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
pub enum ExecuteTransactionError {
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Transaction signing error: {0}")]
    SignerError(#[from] SignerError),
    #[error("Meta-signing error: {0}")]
    MetaSignError(#[from] MetaSignError),
    #[error("Pre-query error: {0}")]
    PreQueryError(#[from] QueryError<RpcQueryRequest>),
    #[error("Retries exhausted. The last error is: {0}")]
    RetriesExhausted(JsonRpcError<RpcTransactionError>),
    #[error("Transaction error: {0}")]
    CriticalTransactionError(JsonRpcError<RpcTransactionError>),
    #[error(transparent)]
    NonEmptyVecError(#[from] NonEmptyVecError),
}

#[derive(thiserror::Error, Debug)]
pub enum ExecuteMetaTransactionsError {
    #[error("Transaction validation error: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Meta-signing error: {0}")]
    SignError(#[from] MetaSignError),
    #[error("Pre-query error: {0}")]
    PreQueryError(#[from] QueryError<RpcQueryRequest>),

    #[error("Relayer is not defined in the network config")]
    RelayerIsNotDefined,

    #[error("Failed to send meta-transaction: {0}")]
    SendError(#[from] reqwest::Error),

    #[error(transparent)]
    NonEmptyVecError(#[from] NonEmptyVecError),
}

#[derive(thiserror::Error, Debug)]
pub enum FTValidatorError {
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
    #[error("Query error: {0}")]
    QueryError(#[from] QueryError<RpcQueryRequest>),

    #[error("Query creation error: {0}")]
    QueryBuilderError(#[from] BuilderError),

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
