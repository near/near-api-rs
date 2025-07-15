use std::sync::Arc;

use near_openapi_client::types::{
    FinalExecutionOutcomeView, JsonRpcRequestForSendTx,
    JsonRpcResponseForRpcTransactionResponseAndRpcError, RpcSendTransactionRequest,
    RpcTransactionResponse,
};

use near_types::{
    BlockHeight, CryptoHash, Nonce, PublicKey, TxExecutionStatus,
    transaction::{
        PrepopulateTransaction, SignedTransaction,
        delegate_action::{SignedDelegateAction, SignedDelegateActionAsBase64},
        result::ExecutionFinalResult,
    },
};
use reqwest::Response;
use tracing::{debug, info};

use crate::{
    common::utils::is_critical_transaction_error,
    config::{NetworkConfig, RetryResponse, retry},
    errors::{
        ExecuteMetaTransactionsError, ExecuteTransactionError, MetaSignError, SendRequestError,
        SignerError, ValidationError,
    },
    signer::Signer,
};

use super::META_TRANSACTION_VALID_FOR_DEFAULT;

const TX_EXECUTOR_TARGET: &str = "near_api::tx::executor";
const META_EXECUTOR_TARGET: &str = "near_api::meta::executor";

#[async_trait::async_trait]
pub trait Transactionable: Send + Sync {
    fn prepopulated(&self) -> PrepopulateTransaction;
    /// Validate the transaction before sending it to the network
    async fn validate_with_network(&self, network: &NetworkConfig) -> Result<(), ValidationError>;

    /// Edit the transaction before sending it to the network.
    /// This is useful for example to add storage deposit to the transaction
    /// if it's needed.
    /// Though, it won't be called if the user has presigned the transaction.
    async fn edit_with_network(&mut self, _network: &NetworkConfig) -> Result<(), ValidationError> {
        Ok(())
    }
}

pub enum TransactionableOrSigned<Signed> {
    /// A transaction that is not signed.
    Transactionable(Box<dyn Transactionable + 'static>),
    /// A transaction that is signed and ready to be sent to the network.
    Signed((Signed, Box<dyn Transactionable + 'static>)),
}

impl<Signed> TransactionableOrSigned<Signed> {
    pub fn signed(self) -> Option<Signed> {
        match self {
            Self::Signed((signed, _)) => Some(signed),
            Self::Transactionable(_) => None,
        }
    }
}

impl<S> TransactionableOrSigned<S> {
    pub fn transactionable(self) -> Box<dyn Transactionable> {
        match self {
            Self::Transactionable(tr) => tr,
            Self::Signed((_, tr)) => tr,
        }
    }
}

/// The handler for signing and sending transaction to the network.
///
/// This is the main entry point for the transaction sending functionality.
pub struct ExecuteSignedTransaction {
    /// The transaction that is either not signed yet or already signed.
    pub tr: TransactionableOrSigned<SignedTransaction>,
    /// The signer that will be used to sign the transaction.
    pub signer: Arc<Signer>,

    pub wait_until: TxExecutionStatus,
}

impl ExecuteSignedTransaction {
    pub fn new<T: Transactionable + 'static>(tr: T, signer: Arc<Signer>) -> Self {
        Self {
            tr: TransactionableOrSigned::Transactionable(Box::new(tr)),
            signer,
            wait_until: TxExecutionStatus::Final,
        }
    }

    /// Changes the transaction to a [meta transaction](https://docs.near.org/concepts/abstraction/meta-transactions), allowing some 3rd party entity to execute it and
    /// pay for the gas.
    ///
    /// Please note, that if you already presigned the transaction, it would require you to sign it again as a meta transaction
    /// is a different type of transaction.
    pub fn meta(self) -> ExecuteMetaTransaction {
        ExecuteMetaTransaction::from_box(self.tr.transactionable(), self.signer)
    }

    pub fn wait_until(mut self, wait_until: TxExecutionStatus) -> Self {
        self.wait_until = wait_until;
        self
    }

    /// Signs the transaction offline without fetching the nonce or block hash from the network.
    ///
    /// The transaction won't be broadcasted to the network and just stored signed in the [Self::tr] struct variable.
    ///
    /// This is useful if you already have the nonce and block hash, or if you want to sign the transaction
    /// offline. Please note that incorrect nonce will lead to transaction failure.
    pub async fn presign_offline(
        mut self,
        public_key: PublicKey,
        block_hash: CryptoHash,
        nonce: Nonce,
    ) -> Result<Self, SignerError> {
        let tr = match &self.tr {
            TransactionableOrSigned::Transactionable(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let signed_tr = self
            .signer
            .sign(tr.prepopulated(), public_key.clone(), nonce, block_hash)
            .await?;

        self.tr = TransactionableOrSigned::Signed((signed_tr, self.tr.transactionable()));
        Ok(self)
    }

    /// Signs the transaction with the custom network configuration but doesn't broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// This is useful if you want to sign with non-default network configuration (e.g, custom RPC URL, sandbox).
    /// The provided call will fetch the nonce and block hash from the given network.
    pub async fn presign_with(
        self,
        network: &NetworkConfig,
    ) -> Result<Self, ExecuteTransactionError> {
        let tr = match &self.tr {
            TransactionableOrSigned::Transactionable(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let signer_key = self.signer.get_public_key().await?;
        let tr = tr.prepopulated();
        let (nonce, hash, _) = self
            .signer
            .fetch_tx_nonce(tr.signer_id.clone(), signer_key.clone(), network)
            .await
            .map_err(MetaSignError::from)?;
        Ok(self.presign_offline(signer_key, hash, nonce).await?)
    }

    /// Signs the transaction with the default mainnet configuration. Does not broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// The provided call will fetch the nonce and block hash from the network.
    pub async fn presign_with_mainnet(self) -> Result<Self, ExecuteTransactionError> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    /// Signs the transaction with the default testnet configuration. Does not broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// The provided call will fetch the nonce and block hash from the network.
    pub async fn presign_with_testnet(self) -> Result<Self, ExecuteTransactionError> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

    /// Sends the transaction to the custom provided network.
    ///
    /// This is useful if you want to send the transaction to a non-default network configuration (e.g, custom RPC URL, sandbox).
    /// Please note that if the transaction is not presigned, it will be signed with the network's nonce and block hash.
    pub async fn send_to(
        mut self,
        network: &NetworkConfig,
    ) -> Result<ExecutionFinalResult, ExecuteTransactionError> {
        let (signed, transactionable) = match &mut self.tr {
            TransactionableOrSigned::Transactionable(tr) => {
                debug!(target: TX_EXECUTOR_TARGET, "Preparing unsigned transaction");
                (None, tr)
            }
            TransactionableOrSigned::Signed((s, tr)) => {
                debug!(target: TX_EXECUTOR_TARGET, "Using pre-signed transaction");
                (Some(s.clone()), tr)
            }
        };

        let wait_until = self.wait_until;

        if signed.is_none() {
            debug!(target: TX_EXECUTOR_TARGET, "Editing transaction with network config");
            transactionable.edit_with_network(network).await?;
        } else {
            debug!(target: TX_EXECUTOR_TARGET, "Validating pre-signed transaction with network config");
            transactionable.validate_with_network(network).await?;
        }

        let signed = match signed {
            Some(s) => s,
            None => {
                debug!(target: TX_EXECUTOR_TARGET, "Signing transaction");
                self.presign_with(network)
                    .await?
                    .tr
                    .signed()
                    .expect("Expect to have it signed")
            }
        };

        info!(
            target: TX_EXECUTOR_TARGET,
            "Broadcasting signed transaction. Hash: {:?}, Signer: {:?}, Receiver: {:?}, Nonce: {}",
            signed.get_hash(),
            signed.transaction.signer_id(),
            signed.transaction.receiver_id(),
            signed.transaction.nonce(),
        );

        Self::send_impl(network, signed, wait_until).await
    }

    /// Sends the transaction to the default mainnet configuration.
    ///
    /// Please note that this will sign the transaction with the mainnet's nonce and block hash if it's not presigned yet.
    pub async fn send_to_mainnet(self) -> Result<ExecutionFinalResult, ExecuteTransactionError> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

    /// Sends the transaction to the default testnet configuration.
    ///
    /// Please note that this will sign the transaction with the testnet's nonce and block hash if it's not presigned yet.
    pub async fn send_to_testnet(self) -> Result<ExecutionFinalResult, ExecuteTransactionError> {
        let network = NetworkConfig::testnet();
        self.send_to(&network).await
    }

    async fn send_impl(
        network: &NetworkConfig,
        signed_tr: SignedTransaction,
        wait_until: TxExecutionStatus,
    ) -> Result<ExecutionFinalResult, ExecuteTransactionError> {
        let hash = signed_tr.get_hash();
        let signed_tx_base64: near_openapi_client::types::SignedTransaction = signed_tr.into();
        let result = retry(network.clone(), |client| {
            let signed_tx_base64 = signed_tx_base64.clone();
            async move {
                let result = match client
                    .send_tx(&JsonRpcRequestForSendTx {
                        id: "0".to_string(),
                        jsonrpc: "2.0".to_string(),
                        method: near_openapi_client::types::JsonRpcRequestForSendTxMethod::SendTx,
                        params: RpcSendTransactionRequest {
                            signed_tx_base64,
                            wait_until,
                        },
                    })
                    .await
                {
                    Ok(result) => match result.into_inner() {
                        JsonRpcResponseForRpcTransactionResponseAndRpcError::Variant0 {
                            result,
                            ..
                        } => RetryResponse::Ok(result),
                        JsonRpcResponseForRpcTransactionResponseAndRpcError::Variant1 {
                            error,
                            ..
                        } => {
                            if is_critical_transaction_error(&error) {
                                RetryResponse::Critical(SendRequestError::ServerError(error))
                            } else {
                                RetryResponse::Retry(SendRequestError::ServerError(error))
                            }
                        }
                    },
                    Err(err) => RetryResponse::Critical(SendRequestError::ClientError(err)),
                };

                tracing::debug!(
                    target: TX_EXECUTOR_TARGET,
                    "Broadcasting transaction {} resulted in {:?}",
                    hash,
                    result
                );

                result
            }
        })
        .await
        .map_err(ExecuteTransactionError::TransactionError)?;

        // TODO: check if we need to add support for that final_execution_status
        let final_execution_outcome_view = match result {
            // We don't use `experimental_tx`, so we can ignore that, but just to be safe
            RpcTransactionResponse::Variant0 {
                final_execution_status: _,
                receipts: _,
                receipts_outcome,
                status,
                transaction,
                transaction_outcome,
            } => FinalExecutionOutcomeView {
                receipts_outcome,
                status,
                transaction,
                transaction_outcome,
            },
            RpcTransactionResponse::Variant1 {
                final_execution_status: _,
                receipts_outcome,
                status,
                transaction,
                transaction_outcome,
            } => FinalExecutionOutcomeView {
                receipts_outcome,
                status,
                transaction,
                transaction_outcome,
            },
        };

        Ok(ExecutionFinalResult::try_from(
            final_execution_outcome_view,
        )?)
    }
}

pub struct ExecuteMetaTransaction {
    pub tr: TransactionableOrSigned<SignedDelegateAction>,
    pub signer: Arc<Signer>,
    pub tx_live_for: Option<BlockHeight>,
}

impl ExecuteMetaTransaction {
    pub fn new<T: Transactionable + 'static>(tr: T, signer: Arc<Signer>) -> Self {
        Self {
            tr: TransactionableOrSigned::Transactionable(Box::new(tr)),
            signer,
            tx_live_for: None,
        }
    }

    pub fn from_box(tr: Box<dyn Transactionable + 'static>, signer: Arc<Signer>) -> Self {
        Self {
            tr: TransactionableOrSigned::Transactionable(tr),
            signer,
            tx_live_for: None,
        }
    }

    /// Sets the transaction live for the given block amount.
    ///
    /// This is useful if you want to set the transaction to be valid for a specific amount of blocks.\
    /// The default amount is 1000 blocks.
    pub const fn tx_live_for(mut self, tx_live_for: BlockHeight) -> Self {
        self.tx_live_for = Some(tx_live_for);
        self
    }

    /// Signs the transaction offline without fetching the nonce or block hash from the network. Does not broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// This is useful if you already have the nonce and block hash, or if you want to sign the transaction
    /// offline. Please note that incorrect nonce will lead to transaction failure and incorrect block height
    /// will lead to incorrectly populated transaction live value.
    pub async fn presign_offline(
        mut self,
        signer_key: PublicKey,
        block_hash: CryptoHash,
        nonce: Nonce,
        block_height: BlockHeight,
    ) -> Result<Self, ExecuteMetaTransactionsError> {
        let tr = match &self.tr {
            TransactionableOrSigned::Transactionable(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let max_block_height = block_height
            + self
                .tx_live_for
                .unwrap_or(META_TRANSACTION_VALID_FOR_DEFAULT);

        let signed_tr = self
            .signer
            .sign_meta(
                tr.prepopulated(),
                signer_key,
                nonce,
                block_hash,
                max_block_height,
            )
            .await?;

        self.tr = TransactionableOrSigned::Signed((signed_tr, self.tr.transactionable()));
        Ok(self)
    }

    /// Signs the transaction with the custom network configuration but doesn't broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// This is useful if you want to sign with non-default network configuration (e.g, custom RPC URL, sandbox).
    pub async fn presign_with(
        self,
        network: &NetworkConfig,
    ) -> Result<Self, ExecuteMetaTransactionsError> {
        let tr = match &self.tr {
            TransactionableOrSigned::Transactionable(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let signer_key = self
            .signer
            .get_public_key()
            .await
            .map_err(MetaSignError::from)?;
        let (nonce, block_hash, block_height) = self
            .signer
            .fetch_tx_nonce(
                tr.prepopulated().signer_id.clone(),
                signer_key.clone(),
                network,
            )
            .await
            .map_err(MetaSignError::from)?;
        self.presign_offline(signer_key, block_hash, nonce, block_height)
            .await
    }

    /// Signs the transaction with the default mainnet configuration but doesn't broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// The provided call will fetch the nonce and block hash, block height from the network.
    pub async fn presign_with_mainnet(self) -> Result<Self, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    /// Signs the transaction with the default testnet configuration but doesn't broadcast it.
    ///
    /// Signed transaction is stored in the [Self::tr] struct variable.
    ///
    /// The provided call will fetch the nonce and block hash, block height from the network.
    pub async fn presign_with_testnet(self) -> Result<Self, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

    /// Sends the transaction to the custom provided network.
    ///
    /// This is useful if you want to send the transaction to a non-default network configuration (e.g, custom RPC URL, sandbox).
    /// Please note that if the transaction is not presigned, it will be sign with the network's nonce and block hash.
    pub async fn send_to(
        mut self,
        network: &NetworkConfig,
    ) -> Result<Response, ExecuteMetaTransactionsError> {
        let (signed, transactionable) = match &mut self.tr {
            TransactionableOrSigned::Transactionable(tr) => {
                debug!(target: META_EXECUTOR_TARGET, "Preparing unsigned meta transaction");
                (None, tr)
            }
            TransactionableOrSigned::Signed((s, tr)) => {
                debug!(target: META_EXECUTOR_TARGET, "Using pre-signed meta transaction");
                (Some(s.clone()), tr)
            }
        };

        if signed.is_none() {
            debug!(target: META_EXECUTOR_TARGET, "Editing meta transaction with network config");
            transactionable.edit_with_network(network).await?;
        } else {
            debug!(target: META_EXECUTOR_TARGET, "Validating pre-signed meta transaction with network config");
            transactionable.validate_with_network(network).await?;
        }

        let signed = match signed {
            Some(s) => s,
            None => {
                debug!(target: META_EXECUTOR_TARGET, "Signing meta transaction");
                self.presign_with(network)
                    .await?
                    .tr
                    .signed()
                    .expect("Expect to have it signed")
            }
        };

        info!(
            target: META_EXECUTOR_TARGET,
            "Broadcasting signed meta transaction. Signer: {:?}, Receiver: {:?}, Nonce: {}, Valid until: {}",
            signed.delegate_action.sender_id,
            signed.delegate_action.receiver_id,
            signed.delegate_action.nonce,
            signed.delegate_action.max_block_height
        );

        Self::send_impl(network, signed).await
    }

    /// Sends the transaction to the default mainnet configuration.
    ///
    /// Please note that this will sign the transaction with the mainnet's nonce and block hash if it's not presigned yet.
    pub async fn send_to_mainnet(self) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

    /// Sends the transaction to the default testnet configuration.
    ///
    /// Please note that this will sign the transaction with the testnet's nonce and block hash if it's not presigned yet.
    pub async fn send_to_testnet(self) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::testnet();
        self.send_to(&network).await
    }

    async fn send_impl(
        network: &NetworkConfig,
        tr: SignedDelegateAction,
    ) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        let client = reqwest::Client::new();
        let json_payload = serde_json::json!({
            "signed_delegate_action": SignedDelegateActionAsBase64::from(
                tr.clone()
            ).to_string(),
        });
        debug!(
            target: META_EXECUTOR_TARGET,
            "Sending meta transaction to relayer. Payload: {:?}",
            json_payload
        );
        let resp = client
            .post(
                network
                    .meta_transaction_relayer_url
                    .clone()
                    .ok_or(ExecuteMetaTransactionsError::RelayerIsNotDefined)?,
            )
            .json(&json_payload)
            .send()
            .await?;

        info!(
            target: META_EXECUTOR_TARGET,
            "Meta transaction sent to relayer. Status: {}, Signer: {:?}, Receiver: {:?}",
            resp.status(),
            tr.delegate_action.sender_id,
            tr.delegate_action.receiver_id
        );
        Ok(resp)
    }
}
