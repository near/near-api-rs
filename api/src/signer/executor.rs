use std::{ops::Deref, sync::Arc};

use near_api_types::{
    AccountId, PublicKey, TxExecutionStatus, transaction::PrepopulateTransaction,
};

use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, warn};

use crate::{
    Signer,
    advanced::TxExecutionResult,
    config::NetworkConfig,
    errors::{ExecuteTransactionError, SignerError},
    signer::TransactionGroupKey,
};

#[allow(async_fn_in_trait)]
pub trait TxExecutor {
    async fn sign_and_send(
        &self,
        account_id: impl Into<AccountId>,
        network: impl Into<NetworkConfig>,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> TxExecutionResult;
}

pub enum TxType {
    Transaction(PrepopulateTransaction),
    TransactionMeta(PrepopulateTransaction),
}

struct TxJob {
    pub account_id: AccountId,
    pub network: NetworkConfig,
    pub transaction: TxType,
    pub wait_until: TxExecutionStatus,
    pub response_sender: oneshot::Sender<TxExecutionResult>,
}

/// A [SequentialSigner](`SequentialSigner`) is a wrapper around a [Signer](`Signer`)
/// that allows to execute transactions sequentially for the tx group
pub struct SequentialSigner {
    signer: Arc<Signer>,
    sequential_channels: dashmap::DashMap<TransactionGroupKey, mpsc::UnboundedSender<TxJob>>,
}

impl SequentialSigner {
    pub async fn new(signer: Signer) -> Self {
        Self {
            signer: Arc::new(signer),
            sequential_channels: dashmap::DashMap::new(),
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
                let signer = self.signer.clone();

                tokio::task::spawn(
                    async move { signer.process_tx_group_sequential(receiver).await },
                );

                sender
            })
            .clone()
    }

    async fn execute_sequentially(
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
            wait_until,
            response_sender,
        };

        channel.send(job).map_err(|e| {
            ExecuteTransactionError::SignerError(SignerError::SequentialSignerError(e.into()))
        })?;

        response_receiver.await.map_err(|e| {
            ExecuteTransactionError::SignerError(SignerError::SequentialSignerError(e.into()))
        })?
    }
}

impl TxExecutor for SequentialSigner {
    #[instrument(skip(self, network, transaction, account_id))]
    async fn sign_and_send(
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

        self.execute_sequentially(
            account_id,
            network,
            public_key,
            TxType::Transaction(transaction),
            wait_until,
        )
        .await
    }
}

impl Deref for SequentialSigner {
    type Target = Signer;

    fn deref(&self) -> &Self::Target {
        &self.signer
    }
}

impl From<Signer> for SequentialSigner {
    fn from(signer: Signer) -> Self {
        SequentialSigner {
            signer: Arc::new(signer),
            sequential_channels: dashmap::DashMap::new(),
        }
    }
}

impl Signer {
    async fn process_tx_group_sequential(&self, mut receiver: mpsc::UnboundedReceiver<TxJob>) {
        while let Some(job) = receiver.recv().await {
            let TxJob {
                account_id,
                network,
                transaction,
                wait_until,
                response_sender,
            }: TxJob = job;

            let result = match transaction {
                TxType::Transaction(tx) => {
                    self.sign_and_send(account_id, network, tx, wait_until)
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
