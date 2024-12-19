use std::sync::Arc;

use near_primitives::{action::Action, types::AccountId};

use crate::{
    common::send::{ExecuteSignedTransaction, Transactionable},
    config::NetworkConfig,
    errors::{SignerError, ValidationError},
    signer::Signer,
    types::transactions::PrepopulateTransaction,
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

#[derive(Debug, Clone)]
pub struct ConstructTransaction {
    pub tr: PrepopulateTransaction,
}

impl ConstructTransaction {
    pub const fn new(signer_id: AccountId, receiver_id: AccountId) -> Self {
        Self {
            tr: PrepopulateTransaction {
                signer_id,
                receiver_id,
                actions: Vec::new(),
            },
        }
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.tr.actions.push(action);
        self
    }

    pub fn add_actions(mut self, action: Vec<Action>) -> Self {
        self.tr.actions.extend(action);
        self
    }

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

/// Low-level transaction builder.
///
/// This struct provides a low-level interface for constructing and signing transactions.
/// It is designed to be used in scenarios where more control over the transaction process is required.
///
/// # Examples
///
/// ```rust,no_run
/// use near_api::*;
/// use near_primitives::{action::Action, transaction::TransferAction};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let signer = Signer::new(Signer::from_ledger())?;
///
/// // Construct a transaction to transfer tokens
/// let transaction_result = Transaction::construct(
///     "sender.near".parse()?,
///     "receiver.near".parse()?
/// )
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
#[derive(Clone, Debug)]
pub struct Transaction;

impl Transaction {
    pub const fn construct(signer_id: AccountId, receiver_id: AccountId) -> ConstructTransaction {
        ConstructTransaction::new(signer_id, receiver_id)
    }

    pub async fn sign_transaction(
        unsigned_tx: near_primitives::transaction::Transaction,
        signer: Arc<Signer>,
    ) -> Result<ExecuteSignedTransaction, SignerError> {
        let public_key = unsigned_tx.public_key().clone();
        let block_hash = *unsigned_tx.block_hash();
        let nonce = unsigned_tx.nonce();

        ConstructTransaction::new(
            unsigned_tx.signer_id().clone(),
            unsigned_tx.receiver_id().clone(),
        )
        .add_actions(unsigned_tx.take_actions())
        .with_signer(signer)
        .presign_offline(public_key, block_hash.into(), nonce)
        .await
    }
}
