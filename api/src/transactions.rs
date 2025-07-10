use std::sync::Arc;

use near_types::{AccountId, Action, transactions::PrepopulateTransaction};

use crate::{
    common::send::{ExecuteSignedTransaction, Transactionable},
    config::NetworkConfig,
    errors::ValidationError,
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
    pub tr: PrepopulateTransaction,
}

impl ConstructTransaction {
    /// Pre-populates a transaction with the given signer and receiver IDs.
    pub const fn new(signer_id: AccountId, receiver_id: AccountId) -> Self {
        Self {
            tr: PrepopulateTransaction {
                signer_id,
                receiver_id,
                actions: Vec::new(),
            },
        }
    }

    /// Adds an action to the transaction.
    pub fn add_action(mut self, action: Action) -> Self {
        self.tr.actions.push(action);
        self
    }

    /// Adds multiple actions to the transaction.
    pub fn add_actions(mut self, actions: Vec<Action>) -> Self {
        self.tr.actions.extend(actions);
        self
    }

    /// Signs the transaction with the given signer.
    pub fn with_signer(self, signer: Arc<Signer>) -> ExecuteSignedTransaction {
        ExecuteSignedTransaction::new(self, signer)
    }
}

#[async_trait::async_trait]
impl Transactionable for ConstructTransaction {
    fn prepopulated(&self) -> PrepopulateTransaction {
        PrepopulateTransaction {
            signer_id: self.tr.signer_id.clone(),
            receiver_id: self.tr.receiver_id.clone(),
            actions: self.tr.actions.clone(),
        }
    }

    async fn validate_with_network(&self, _: &NetworkConfig) -> Result<(), ValidationError> {
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
    /// use near_api::*;
    /// use near_primitives::transaction::{Action, TransferAction};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    ///
    /// let transaction_result: near_primitives::views::FinalExecutionOutcomeView = Transaction::construct(
    ///     "sender.near".parse()?,
    ///     "receiver.near".parse()?
    /// )
    /// .add_action(Action::Transfer(
    ///     TransferAction {
    ///         deposit: NearToken::from_near(1).as_yoctonear(),
    ///     },
    /// ))
    /// .add_action(Action::Transfer(
    ///     TransferAction {
    ///         deposit: NearToken::from_near(1).as_yoctonear(),
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
    /// use near_primitives::transaction::{Action, TransferAction};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// # let unsigned_tx = todo!();
    ///
    /// let transaction_result: near_primitives::views::FinalExecutionOutcomeView = Transaction::sign_transaction(
    ///     unsigned_tx,
    ///     signer
    /// )
    /// .await?
    /// .send_to_mainnet()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn use_transaction(
        unsigned_tx: PrepopulateTransaction,
        signer: Arc<Signer>,
    ) -> ExecuteSignedTransaction {
        ConstructTransaction::new(unsigned_tx.signer_id, unsigned_tx.receiver_id)
            .add_actions(unsigned_tx.actions)
            .with_signer(signer)
    }

    // TODO: fetch transaction status
    // TODO: fetch transaction receipt
    // TODO: fetch transaction proof
}
