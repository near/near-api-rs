use near_api_types::{
    AccountId, PublicKey, TxExecutionStatus,
    transaction::{PrepopulateTransaction, SignedTransaction, result::ExecutionFinalResult},
};

use tokio::sync::{mpsc, oneshot};
use tracing::{debug, instrument, warn};

use crate::{
    Signer,
    advanced::{ExecuteSignedTransaction, TxExecutionResult},
    config::NetworkConfig,
    errors::{ExecuteTransactionError, SignerError},
    signer::{InnerSigner, SIGNER_TARGET, TransactionGroupKey},
};

pub enum TxType {
    Transaction(PrepopulateTransaction),
    MetaTransaction(PrepopulateTransaction),
}

pub(crate) struct TxJob {
    pub account_id: AccountId,
    pub network: NetworkConfig,
    pub transaction: TxType,
    pub response_sender:
        oneshot::Sender<Result<(SignedTransaction, ExecutionFinalResult), ExecuteTransactionError>>,
}

impl Signer {
    #[instrument(skip(self, network, transaction, account_id))]
    pub async fn sign_and_send(
        &self,
        account_id: impl Into<AccountId>,
        network: impl Into<NetworkConfig>,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> TxExecutionResult {
        let public_key = self
            .get_public_key()
            .await
            .map_err(|e| ExecuteTransactionError::SignerError(SignerError::PublicKeyError(e)))?;

        if self
            .sequential_mode
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            self.broadcast_tx_sequentially(
                account_id,
                network,
                public_key,
                TxType::Transaction(transaction),
                wait_until,
            )
            .await
        } else {
            let (_, execution_result) = self
                .broadcast_tx(account_id, network, transaction, wait_until)
                .await?;
            Ok(execution_result)
        }
    }

    async fn broadcast_tx_sequentially(
        &self,
        account_id: impl Into<AccountId>,
        network: impl Into<NetworkConfig>,
        public_key: PublicKey,
        transaction: TxType,
        wait_until: TxExecutionStatus,
    ) -> TxExecutionResult {
        let account_id = account_id.into();
        let network = network.into();

        let key = (account_id.clone(), public_key, network.network_name.clone());
        let channel = self.get_tx_group_channel(key).await;
        let (response_sender, response_receiver) = oneshot::channel();

        let job: TxJob = TxJob {
            account_id,
            network: network.clone(),
            transaction,
            response_sender,
        };

        channel.send(job).map_err(|e| {
            ExecuteTransactionError::SignerError(SignerError::SequentialSignerError(e.into()))
        })?;

        let (signed, execution_result) = response_receiver.await.map_err(|e| {
            ExecuteTransactionError::SignerError(SignerError::SequentialSignerError(e.into()))
        })??;

        match wait_until {
            TxExecutionStatus::Included => Ok(execution_result),
            _ => ExecuteSignedTransaction::fetch_tx(network, signed, wait_until).await,
        }
    }

    async fn get_tx_group_channel(
        &self,
        tx_group_key: TransactionGroupKey,
    ) -> mpsc::UnboundedSender<TxJob> {
        self.sequential_channels
            .entry(tx_group_key)
            .or_insert_with(|| {
                let (sender, receiver) = mpsc::unbounded_channel::<TxJob>();
                let signer = self.inner.clone();

                tokio::task::spawn(
                    async move { signer.process_tx_group_sequential(receiver).await },
                );

                sender
            })
            .clone()
    }
}

impl InnerSigner {
    /// Signs and sends a transaction to the network.
    /// This method combines the signing and sending steps, and also manages the nonce
    /// fetching and caching.
    ///
    /// This method does not wait for the transaction to be included in a block,
    /// it only ensures that the transaction is sent to the network.
    #[instrument(skip(self, account_id, network))]
    async fn broadcast_tx(
        &self,
        account_id: impl Into<AccountId>,
        network: impl Into<NetworkConfig>,
        transaction: PrepopulateTransaction,
        wait_untill: TxExecutionStatus,
    ) -> Result<(SignedTransaction, ExecutionFinalResult), ExecuteTransactionError> {
        debug!(target: SIGNER_TARGET, "Sending transaction");

        let account_id = account_id.into();
        let network = network.into();

        let public_key = self.get_public_key().await.map_err(SignerError::from)?;

        let (nonce, block_hash, _) = self
            .fetch_tx_nonce(account_id, public_key, &network)
            .await?;

        let signed_transaction = self
            .sign(transaction, public_key, nonce, block_hash)
            .await?;

        let execution_result =
            ExecuteSignedTransaction::send_impl(network, signed_transaction.clone(), wait_untill)
                .await?;

        Ok((signed_transaction, execution_result))
    }

    /// This method handles the sequential execution if enabled, by acquiring a lock for the
    /// specific (account_id, public_key, network) group.
    #[instrument(skip(self, receiver))]
    async fn process_tx_group_sequential(&self, mut receiver: mpsc::UnboundedReceiver<TxJob>) {
        while let Some(job) = receiver.recv().await {
            let TxJob {
                account_id,
                network,
                transaction,
                response_sender,
            }: TxJob = job;

            // Waiting for transaction to be sent to be included in a block
            // before processing the next one to ensure sequential execution.
            let result = match transaction {
                TxType::Transaction(tx) => {
                    self.broadcast_tx(account_id, network, tx, TxExecutionStatus::Included)
                        .await
                }
                _ => unimplemented!("Meta transactions are not implemented yet"),
            };

            response_sender.send(result).unwrap_or_else(|e| {
                warn!("Failed to send transaction execution result: {:?}", e);
            });
        }
    }
}
