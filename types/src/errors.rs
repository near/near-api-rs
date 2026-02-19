use std::array::TryFromSliceError;

use crate::transaction::result::ExecutionFailure;

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
pub enum KeyTypeError {
    #[error("Invalid key format. Expected: [ed25519, secp256k1] but got: {0}")]
    InvalidKeyFormat(String),
    #[error("Invalid key type byte index: {0}")]
    InvalidKeyTypeByteIndex(u8),
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
    InvalidKeyFormat(#[from] KeyTypeError),
    #[error("Delegate action is not supported")]
    DelegateActionNotSupported,
    #[error("Invalid global contract identifier")]
    InvalidGlobalContractIdentifier,
}

impl From<Vec<u8>> for DataConversionError {
    fn from(value: Vec<u8>) -> Self {
        Self::IncorrectLength(value.len())
    }
}

impl From<TryFromSliceError> for DataConversionError {
    fn from(_: TryFromSliceError) -> Self {
        Self::IncorrectLength(0)
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
    #[error("Executing transaction failed: {0}")]
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

#[derive(thiserror::Error, Debug)]
pub enum SecretKeyError {
    #[error("Invalid secret key: {0}")]
    InvalidSecp256k1SecretKey(secp256k1::Error),
    #[error("Invalid ed25519 secret key: {0}")]
    InvalidEd25519SecretKey(ed25519_dalek::SignatureError),
    #[error("Invalid conversion: {0}")]
    InvalidConversion(#[from] DataConversionError),
}

impl From<ed25519_dalek::SignatureError> for SecretKeyError {
    fn from(value: ed25519_dalek::SignatureError) -> Self {
        Self::InvalidEd25519SecretKey(value)
    }
}

impl From<secp256k1::Error> for SecretKeyError {
    fn from(value: secp256k1::Error) -> Self {
        Self::InvalidSecp256k1SecretKey(value)
    }
}

impl From<Vec<u8>> for SecretKeyError {
    fn from(value: Vec<u8>) -> Self {
        Self::InvalidConversion(value.into())
    }
}

impl From<TryFromSliceError> for SecretKeyError {
    fn from(error: TryFromSliceError) -> Self {
        Self::InvalidConversion(error.into())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SignatureErrors {
    #[error("Invalid signature data: {0}")]
    InvalidSignatureData(secp256k1::Error),
}

impl From<secp256k1::Error> for SignatureErrors {
    fn from(value: secp256k1::Error) -> Self {
        Self::InvalidSignatureData(value)
    }
}

// -- Structured transaction error types --
// These mirror `near_openapi_types` types but parse contract panic messages
// into structured `ContractPanicError(serde_json::Value)`.

#[derive(thiserror::Error, Debug, Clone)]
pub enum FunctionCallError {
    #[error("WasmUnknownError")]
    WasmUnknownError,
    #[error("EvmError")]
    EvmError,
    #[error("CompilationError({0})")]
    CompilationError(near_openapi_types::CompilationError),
    #[error("LinkError({msg})")]
    LinkError { msg: String },
    #[error("MethodResolveError({0})")]
    MethodResolveError(near_openapi_types::MethodResolveError),
    #[error("WasmTrap({0})")]
    WasmTrap(near_openapi_types::WasmTrap),
    #[error("HostError({0})")]
    HostError(near_openapi_types::HostError),
    #[error("ExecutionError({0})")]
    ExecutionError(String),
    #[error("ContractPanicError({0})")]
    ContractPanicError(serde_json::Value),
}

impl From<near_openapi_types::FunctionCallError> for FunctionCallError {
    fn from(err: near_openapi_types::FunctionCallError) -> Self {
        use near_openapi_types::FunctionCallError as Ext;
        match err {
            Ext::HostError(near_openapi_types::HostError::GuestPanic { panic_msg }) => {
                match serde_json::from_str::<serde_json::Value>(&panic_msg) {
                    Ok(value) => Self::ContractPanicError(value),
                    Err(_) => {
                        Self::HostError(near_openapi_types::HostError::GuestPanic { panic_msg })
                    }
                }
            }
            Ext::ExecutionError(msg) => {
                if let Some(json_str) = msg.strip_prefix("Smart contract panicked: ") {
                    if let Ok(value) = serde_json::from_str(json_str) {
                        return Self::ContractPanicError(value);
                    }
                }
                Self::ExecutionError(msg)
            }
            Ext::WasmUnknownError => Self::WasmUnknownError,
            Ext::EvmError => Self::EvmError,
            Ext::CompilationError(e) => Self::CompilationError(e),
            Ext::LinkError { msg } => Self::LinkError { msg },
            Ext::MethodResolveError(e) => Self::MethodResolveError(e),
            Ext::WasmTrap(e) => Self::WasmTrap(e),
            Ext::HostError(e) => Self::HostError(e),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum ActionErrorKind {
    #[error("Account {account_id} already exists")]
    AccountAlreadyExists {
        account_id: near_account_id::AccountId,
    },
    #[error("Account {account_id} does not exist")]
    AccountDoesNotExist {
        account_id: near_account_id::AccountId,
    },
    #[error("Account {account_id} can only be created by registrar {registrar_account_id}")]
    CreateAccountOnlyByRegistrar {
        account_id: near_account_id::AccountId,
        predecessor_id: near_account_id::AccountId,
        registrar_account_id: near_account_id::AccountId,
    },
    #[error("Account {account_id} cannot be created by {predecessor_id}")]
    CreateAccountNotAllowed {
        account_id: near_account_id::AccountId,
        predecessor_id: near_account_id::AccountId,
    },
    #[error("Actor {actor_id} has no permission on account {account_id}")]
    ActorNoPermission {
        account_id: near_account_id::AccountId,
        actor_id: near_account_id::AccountId,
    },
    #[error("Delete key does not exist for account {account_id}")]
    DeleteKeyDoesNotExist {
        account_id: near_account_id::AccountId,
        public_key: near_openapi_types::PublicKey,
    },
    #[error("Add key already exists for account {account_id}")]
    AddKeyAlreadyExists {
        account_id: near_account_id::AccountId,
        public_key: near_openapi_types::PublicKey,
    },
    #[error("Account {account_id} is staking and cannot be deleted")]
    DeleteAccountStaking {
        account_id: near_account_id::AccountId,
    },
    #[error("Account {account_id} lacks balance for state, needs {amount}")]
    LackBalanceForState {
        account_id: near_account_id::AccountId,
        amount: near_token::NearToken,
    },
    #[error("Account {account_id} is not staked")]
    TriesToUnstake {
        account_id: near_account_id::AccountId,
    },
    #[error("Account {account_id} insufficient balance to stake")]
    TriesToStake {
        account_id: near_account_id::AccountId,
        balance: near_token::NearToken,
        locked: near_token::NearToken,
        stake: near_token::NearToken,
    },
    #[error("Account {account_id} insufficient stake")]
    InsufficientStake {
        account_id: near_account_id::AccountId,
        minimum_stake: near_token::NearToken,
        stake: near_token::NearToken,
    },
    #[error("{0}")]
    FunctionCallError(#[from] FunctionCallError),
    #[error("New receipt validation error: {0}")]
    NewReceiptValidationError(near_openapi_types::ReceiptValidationError),
    #[error("Only implicit account creation allowed for {account_id}")]
    OnlyImplicitAccountCreationAllowed {
        account_id: near_account_id::AccountId,
    },
    #[error("Delete account with large state: {account_id}")]
    DeleteAccountWithLargeState {
        account_id: near_account_id::AccountId,
    },
    #[error("Delegate action has invalid signature")]
    DelegateActionInvalidSignature,
    #[error("Delegate action sender {sender_id} does not match tx receiver {receiver_id}")]
    DelegateActionSenderDoesNotMatchTxReceiver {
        receiver_id: near_account_id::AccountId,
        sender_id: near_account_id::AccountId,
    },
    #[error("Delegate action expired")]
    DelegateActionExpired,
    #[error("Delegate action access key error: {0}")]
    DelegateActionAccessKeyError(near_openapi_types::InvalidAccessKeyError),
    #[error("Delegate action invalid nonce: {delegate_nonce}, access key nonce: {ak_nonce}")]
    DelegateActionInvalidNonce { ak_nonce: u64, delegate_nonce: u64 },
    #[error("Delegate action nonce {delegate_nonce} too large, upper bound: {upper_bound}")]
    DelegateActionNonceTooLarge {
        delegate_nonce: u64,
        upper_bound: u64,
    },
    #[error("Global contract does not exist: {identifier:?}")]
    GlobalContractDoesNotExist {
        identifier: near_openapi_types::GlobalContractIdentifier,
    },
    #[error("Gas key does not exist for account {account_id}")]
    GasKeyDoesNotExist {
        account_id: near_account_id::AccountId,
        public_key: near_openapi_types::PublicKey,
    },
    #[error("Gas key already exists for account {account_id}")]
    GasKeyAlreadyExists {
        account_id: near_account_id::AccountId,
        public_key: near_openapi_types::PublicKey,
    },
}

impl From<near_openapi_types::ActionErrorKind> for ActionErrorKind {
    fn from(kind: near_openapi_types::ActionErrorKind) -> Self {
        use near_openapi_types::ActionErrorKind as Ext;
        match kind {
            Ext::AccountAlreadyExists { account_id } => {
                Self::AccountAlreadyExists { account_id }
            }
            Ext::AccountDoesNotExist { account_id } => {
                Self::AccountDoesNotExist { account_id }
            }
            Ext::CreateAccountOnlyByRegistrar {
                account_id,
                predecessor_id,
                registrar_account_id,
            } => Self::CreateAccountOnlyByRegistrar {
                account_id,
                predecessor_id,
                registrar_account_id,
            },
            Ext::CreateAccountNotAllowed {
                account_id,
                predecessor_id,
            } => Self::CreateAccountNotAllowed {
                account_id,
                predecessor_id,
            },
            Ext::ActorNoPermission {
                account_id,
                actor_id,
            } => Self::ActorNoPermission {
                account_id,
                actor_id,
            },
            Ext::DeleteKeyDoesNotExist {
                account_id,
                public_key,
            } => Self::DeleteKeyDoesNotExist {
                account_id,
                public_key,
            },
            Ext::AddKeyAlreadyExists {
                account_id,
                public_key,
            } => Self::AddKeyAlreadyExists {
                account_id,
                public_key,
            },
            Ext::DeleteAccountStaking { account_id } => {
                Self::DeleteAccountStaking { account_id }
            }
            Ext::LackBalanceForState {
                account_id,
                amount,
            } => Self::LackBalanceForState {
                account_id,
                amount,
            },
            Ext::TriesToUnstake { account_id } => Self::TriesToUnstake { account_id },
            Ext::TriesToStake {
                account_id,
                balance,
                locked,
                stake,
            } => Self::TriesToStake {
                account_id,
                balance,
                locked,
                stake,
            },
            Ext::InsufficientStake {
                account_id,
                minimum_stake,
                stake,
            } => Self::InsufficientStake {
                account_id,
                minimum_stake,
                stake,
            },
            Ext::FunctionCallError(e) => Self::FunctionCallError(e.into()),
            Ext::NewReceiptValidationError(e) => Self::NewReceiptValidationError(e),
            Ext::OnlyImplicitAccountCreationAllowed { account_id } => {
                Self::OnlyImplicitAccountCreationAllowed { account_id }
            }
            Ext::DeleteAccountWithLargeState { account_id } => {
                Self::DeleteAccountWithLargeState { account_id }
            }
            Ext::DelegateActionInvalidSignature => Self::DelegateActionInvalidSignature,
            Ext::DelegateActionSenderDoesNotMatchTxReceiver {
                receiver_id,
                sender_id,
            } => Self::DelegateActionSenderDoesNotMatchTxReceiver {
                receiver_id,
                sender_id,
            },
            Ext::DelegateActionExpired => Self::DelegateActionExpired,
            Ext::DelegateActionAccessKeyError(e) => Self::DelegateActionAccessKeyError(e),
            Ext::DelegateActionInvalidNonce {
                ak_nonce,
                delegate_nonce,
            } => Self::DelegateActionInvalidNonce {
                ak_nonce,
                delegate_nonce,
            },
            Ext::DelegateActionNonceTooLarge {
                delegate_nonce,
                upper_bound,
            } => Self::DelegateActionNonceTooLarge {
                delegate_nonce,
                upper_bound,
            },
            Ext::GlobalContractDoesNotExist { identifier } => {
                Self::GlobalContractDoesNotExist { identifier }
            }
            Ext::GasKeyDoesNotExist {
                account_id,
                public_key,
            } => Self::GasKeyDoesNotExist {
                account_id,
                public_key,
            },
            Ext::GasKeyAlreadyExists {
                account_id,
                public_key,
            } => Self::GasKeyAlreadyExists {
                account_id,
                public_key,
            },
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
#[error("Action #{index:?}: {kind}")]
pub struct ActionError {
    pub index: Option<u64>,
    pub kind: ActionErrorKind,
}

impl From<near_openapi_types::ActionError> for ActionError {
    fn from(err: near_openapi_types::ActionError) -> Self {
        Self {
            index: err.index,
            kind: err.kind.into(),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum TxExecutionError {
    #[error("{0}")]
    ActionError(#[from] ActionError),
    #[error("{0}")]
    InvalidTxError(near_openapi_types::InvalidTxError),
}

impl From<near_openapi_types::TxExecutionError> for TxExecutionError {
    fn from(err: near_openapi_types::TxExecutionError) -> Self {
        match err {
            near_openapi_types::TxExecutionError::ActionError(e) => Self::ActionError(e.into()),
            near_openapi_types::TxExecutionError::InvalidTxError(e) => Self::InvalidTxError(e),
        }
    }
}
