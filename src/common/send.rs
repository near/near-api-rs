use std::sync::Arc;

use near_crypto::PublicKey;
use near_primitives::{
    action::delegate::SignedDelegateAction,
    transaction::SignedTransaction,
    types::{BlockHeight, Nonce},
    views::FinalExecutionOutcomeView,
};
use reqwest::Response;
use tracing::{debug, info};

use crate::{
    config::NetworkConfig,
    errors::{
        ExecuteMetaTransactionsError, ExecuteTransactionError, MetaSignError, SignerError,
        ValidationError,
    },
    signer::Signer,
    types::{transactions::PrepopulateTransaction, CryptoHash},
};

use super::{
    signed_delegate_action::SignedDelegateActionAsBase64, utils::retry,
    META_TRANSACTION_VALID_FOR_DEFAULT,
};

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
    Transactionable(Box<dyn Transactionable + 'static>),
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

impl From<SignedTransaction> for PrepopulateTransaction {
    fn from(tr: SignedTransaction) -> Self {
        Self {
            signer_id: tr.transaction.signer_id().clone(),
            receiver_id: tr.transaction.receiver_id().clone(),
            actions: tr.transaction.take_actions(),
        }
    }
}

pub struct ExecuteSignedTransaction {
    pub tr: TransactionableOrSigned<SignedTransaction>,
    pub signer: Arc<Signer>,
    pub retries: u8,
    pub sleep_duration: std::time::Duration,
    pub exponential_backoff: bool,
}

impl ExecuteSignedTransaction {
    pub fn new<T: Transactionable + 'static>(tr: T, signer: Arc<Signer>) -> Self {
        Self {
            tr: TransactionableOrSigned::Transactionable(Box::new(tr)),
            signer,
            retries: 5,
            // 50ms, 100ms, 200ms, 400ms, 800ms
            sleep_duration: std::time::Duration::from_millis(50),
            exponential_backoff: true,
        }
    }

    pub fn meta(self) -> ExecuteMetaTransaction {
        ExecuteMetaTransaction::from_box(self.tr.transactionable(), self.signer)
    }

    pub const fn with_retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }

    pub const fn with_sleep_duration(mut self, sleep_duration: std::time::Duration) -> Self {
        self.sleep_duration = sleep_duration;
        self
    }

    pub const fn with_exponential_backoff(mut self) -> Self {
        self.exponential_backoff = true;
        self
    }

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

    pub async fn presign_with_mainnet(self) -> Result<Self, ExecuteTransactionError> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    pub async fn presign_with_testnet(self) -> Result<Self, ExecuteTransactionError> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

    pub async fn send_to(
        mut self,
        network: &NetworkConfig,
    ) -> Result<FinalExecutionOutcomeView, ExecuteTransactionError> {
        let sleep_duration = self.sleep_duration;
        let retries = self.retries;

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

        Self::send_impl(network, signed, retries, sleep_duration).await
    }

    pub async fn send_to_mainnet(
        self,
    ) -> Result<FinalExecutionOutcomeView, ExecuteTransactionError> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

    pub async fn send_to_testnet(
        self,
    ) -> Result<FinalExecutionOutcomeView, ExecuteTransactionError> {
        let network = NetworkConfig::testnet();
        self.send_to(&network).await
    }

    async fn send_impl(
        network: &NetworkConfig,
        signed_tr: SignedTransaction,
        retries: u8,
        sleep_duration: std::time::Duration,
    ) -> Result<FinalExecutionOutcomeView, ExecuteTransactionError> {
        retry(
            || {
                let signed_tr = signed_tr.clone();
                async move {
                    let result = network
                .json_rpc_client()
                .call(
                    near_jsonrpc_client::methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
                        signed_transaction: signed_tr.clone(),
                        },
                    )
                    .await;

                    tracing::debug!(
                        target: TX_EXECUTOR_TARGET,
                        "Broadcasting transaction {} resulted in {:?}",
                        signed_tr.get_hash(),
                        result
                    );

                    result
                }
            },
            retries,
            sleep_duration,
            false,
        )
        .await
        .map_err(ExecuteTransactionError::RetriesExhausted)
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

    pub const fn tx_live_for(mut self, tx_live_for: BlockHeight) -> Self {
        self.tx_live_for = Some(tx_live_for);
        self
    }

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
            .await
            .map_err(MetaSignError::from)?;

        self.tr = TransactionableOrSigned::Signed((signed_tr, self.tr.transactionable()));
        Ok(self)
    }

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

    pub async fn presign_with_mainnet(self) -> Result<Self, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    pub async fn presign_with_testnet(self) -> Result<Self, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

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

    pub async fn send_to_mainnet(self) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

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
