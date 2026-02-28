use std::sync::Arc;

use near_api_types::{
    AccountId, Action, CryptoHash, TxExecutionStatus, transaction::PrepopulateTransaction,
};

use crate::{
    common::{
        query::{
            ReceiptHandler, RequestBuilder, TransactionStatusHandler,
            tx_rpc::{
                ReceiptRef, ReceiptRpc, TransactionProofRef, TransactionProofRpc,
                TransactionStatusRef, TransactionStatusRpc,
            },
        },
        send::{ExecuteSignedTransaction, Transactionable},
    },
    config::NetworkConfig,
    errors::{ArgumentValidationError, ValidationError},
    signer::Signer,
};

#[derive(Clone, Debug)]
pub struct TransactionWithSign<T: Transactionable + 'static> {
    pub tx: T,
}

impl<T: Transactionable> TransactionWithSign<T> {
    pub fn with_signer(self, signer: Arc<Signer>) -> ExecuteSignedTransaction {
        ExecuteSignedTransaction::new(self.tx, signer)
    }
}

#[derive(Clone, Debug)]
pub struct SelfActionBuilder {
    pub actions: Vec<Action>,
}

impl Default for SelfActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfActionBuilder {
    pub const fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Adds an action to the transaction.
    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Adds multiple actions to the transaction.
    pub fn add_actions(mut self, actions: Vec<Action>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Signs the transaction with the given account id and signer related to it.
    pub fn with_signer(
        self,
        signer_account_id: AccountId,
        signer: Arc<Signer>,
    ) -> ExecuteSignedTransaction {
        ConstructTransaction::new(signer_account_id.clone(), signer_account_id)
            .add_actions(self.actions)
            .with_signer(signer)
    }
}

/// A builder for constructing transactions using Actions.
#[derive(Debug, Clone)]
pub struct ConstructTransaction {
    pub transaction: Result<PrepopulateTransaction, ArgumentValidationError>,
}

impl ConstructTransaction {
    /// Pre-populates a transaction with the given signer and receiver IDs.
    pub const fn new(signer_id: AccountId, receiver_id: AccountId) -> Self {
        Self {
            transaction: Ok(PrepopulateTransaction {
                signer_id,
                receiver_id,
                actions: Vec::new(),
            }),
        }
    }

    pub fn with_deferred_error(mut self, error: ArgumentValidationError) -> Self {
        self.transaction = Err(error);
        self
    }

    /// Adds an action to the transaction.
    pub fn add_action(mut self, action: Action) -> Self {
        if let Ok(transaction) = &mut self.transaction {
            transaction.actions.push(action);
        }
        self
    }

    /// Adds multiple actions to the transaction.
    pub fn add_actions(mut self, actions: Vec<Action>) -> Self {
        if let Ok(transaction) = &mut self.transaction {
            transaction.actions.extend(actions);
        }
        self
    }

    /// Signs the transaction with the given signer.
    pub fn with_signer(self, signer: Arc<Signer>) -> ExecuteSignedTransaction {
        ExecuteSignedTransaction::new(self, signer)
    }
}

#[async_trait::async_trait]
impl Transactionable for ConstructTransaction {
    fn prepopulated(&self) -> Result<PrepopulateTransaction, ArgumentValidationError> {
        self.transaction.clone()
    }

    async fn validate_with_network(&self, _: &NetworkConfig) -> Result<(), ValidationError> {
        if let Err(e) = &self.transaction {
            return Err(e.to_owned().into());
        }
        Ok(())
    }
}

/// Transaction related functionality.
///
/// This struct provides ability to interact with transactions.
#[derive(Clone, Debug)]
pub struct Transaction;

impl Transaction {
    /// Constructs a new transaction builder with the given signer and receiver IDs.
    /// This pattern is useful for batching actions into a single transaction.
    ///
    /// This is the low level interface for constructing transactions.
    /// It is designed to be used in scenarios where more control over the transaction process is required.
    ///
    /// # Example
    ///
    /// This example constructs a transaction with a two transfer actions.
    ///
    /// ```rust,no_run
    /// use near_api::{*, types::{transaction::actions::{Action, TransferAction}, json::U128}};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::from_ledger()?;
    ///
    /// let transaction_result = Transaction::construct(
    ///     "sender.near".parse()?,
    ///     "receiver.near".parse()?
    /// )
    /// .add_action(Action::Transfer(
    ///     TransferAction {
    ///         deposit: NearToken::from_near(1),
    ///     },
    /// ))
    /// .add_action(Action::Transfer(
    ///     TransferAction {
    ///         deposit: NearToken::from_near(1),
    ///     },
    /// ))
    /// .with_signer(signer)
    /// .send_to_mainnet()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn construct(signer_id: AccountId, receiver_id: AccountId) -> ConstructTransaction {
        ConstructTransaction::new(signer_id, receiver_id)
    }

    /// Signs a transaction with the given signer.
    ///
    /// This provides ability to sign custom constructed pre-populated transactions.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::from_ledger()?;
    /// # let unsigned_tx = todo!();
    ///
    /// let transaction_result = Transaction::use_transaction(
    ///     unsigned_tx,
    ///     signer
    /// )
    /// .send_to_mainnet()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn use_transaction(
        unsigned_tx: PrepopulateTransaction,
        signer: Arc<Signer>,
    ) -> ExecuteSignedTransaction {
        ConstructTransaction::new(unsigned_tx.signer_id, unsigned_tx.receiver_id)
            .add_actions(unsigned_tx.actions)
            .with_signer(signer)
    }

    /// Sets up a query to fetch the current status of a transaction by its hash and sender account ID.
    ///
    /// Returns the transaction status at the current point in time without waiting for
    /// any specific execution stage. If you need to wait until the transaction reaches
    /// a particular stage (e.g., `Final`), use [`Transaction::status_with_options`] instead.
    ///
    /// The returned result is an [`ExecutionFinalResult`](near_api_types::transaction::result::ExecutionFinalResult)
    /// which provides details about gas usage, logs, and the execution status.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash: CryptoHash = "EaNakSaXUTjbPsUJbuDdbuq3e6Ynmjo8zYUgDVqt1iTn".parse()?;
    /// let sender: AccountId = "sender.near".parse()?;
    ///
    /// let result = Transaction::status(sender, tx_hash)
    ///     .fetch_from_mainnet()
    ///     .await?;
    /// println!("Transaction success: {}", result.is_success());
    /// # Ok(())
    /// # }
    /// ```
    pub fn status(
        sender_account_id: AccountId,
        tx_hash: CryptoHash,
    ) -> RequestBuilder<TransactionStatusHandler> {
        Self::status_with_options(sender_account_id, tx_hash, TxExecutionStatus::None)
    }

    /// Sets up a query to fetch the status of a transaction, waiting until it reaches
    /// the specified execution stage.
    ///
    /// Use [`TxExecutionStatus::None`] to return immediately with whatever state is available,
    /// or [`TxExecutionStatus::Final`] to wait until the transaction is fully finalized.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use near_api::{*, types::TxExecutionStatus};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash: CryptoHash = "EaNakSaXUTjbPsUJbuDdbuq3e6Ynmjo8zYUgDVqt1iTn".parse()?;
    /// let sender: AccountId = "sender.near".parse()?;
    ///
    /// let result = Transaction::status_with_options(
    ///     sender,
    ///     tx_hash,
    ///     TxExecutionStatus::Final,
    /// )
    /// .fetch_from_mainnet()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn status_with_options(
        sender_account_id: AccountId,
        tx_hash: CryptoHash,
        wait_until: TxExecutionStatus,
    ) -> RequestBuilder<TransactionStatusHandler> {
        RequestBuilder::new(
            TransactionStatusRpc,
            TransactionStatusRef {
                sender_account_id,
                tx_hash,
                wait_until,
            },
            TransactionStatusHandler,
        )
    }

    /// Sets up a query to fetch a receipt by its ID.
    ///
    /// This uses the `EXPERIMENTAL_receipt` RPC method to retrieve the details of a specific receipt.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let receipt_id: CryptoHash = "EaNakSaXUTjbPsUJbuDdbuq3e6Ynmjo8zYUgDVqt1iTn".parse()?;
    ///
    /// let receipt = Transaction::receipt(receipt_id)
    ///     .fetch_from_mainnet()
    ///     .await?;
    /// println!("Receipt receiver: {:?}", receipt.receiver_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn receipt(receipt_id: CryptoHash) -> RequestBuilder<ReceiptHandler> {
        RequestBuilder::new(ReceiptRpc, ReceiptRef { receipt_id }, ReceiptHandler)
    }

    /// Sets up a query to fetch the light client execution proof for a transaction.
    ///
    /// This is used to verify a transaction's execution against a light client block header.
    /// The `light_client_head` parameter specifies the block hash of the light client's latest known head.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash: CryptoHash = "EaNakSaXUTjbPsUJbuDdbuq3e6Ynmjo8zYUgDVqt1iTn".parse()?;
    /// let sender: AccountId = "sender.near".parse()?;
    /// let head_hash: CryptoHash = "3i1SypXzBRhLMvpHmNJXpg18FgVW6jNFrFcUqBF5Wmit".parse()?;
    ///
    /// let proof = Transaction::proof(sender, tx_hash, head_hash)
    ///     .fetch_from_mainnet()
    ///     .await?;
    /// println!("Proof block header: {:?}", proof.block_header_lite);
    /// # Ok(())
    /// # }
    /// ```
    pub fn proof(
        sender_id: AccountId,
        transaction_hash: CryptoHash,
        light_client_head: CryptoHash,
    ) -> RequestBuilder<TransactionProofRpc> {
        RequestBuilder::new(
            TransactionProofRpc,
            TransactionProofRef {
                sender_id,
                transaction_hash,
                light_client_head,
            },
            TransactionProofRpc,
        )
    }
}
