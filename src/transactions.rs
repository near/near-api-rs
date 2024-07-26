use std::convert::Infallible;

use near_primitives::{action::Action, types::AccountId};

use crate::{
    common::{
        query::QueryBuilder,
        send::{ExecuteSignedTransaction, Transactionable},
    },
    config::NetworkConfig,
    errors::SignerError,
    signer::Signer,
    types::transactions::PrepopulateTransaction,
};

#[derive(Clone, Debug)]
pub struct TransactionWithSign<T: Transactionable> {
    pub tx: T,
}

impl<T: Transactionable> TransactionWithSign<T> {
    pub fn with_signer(self, signer: Signer) -> ExecuteSignedTransaction<T> {
        ExecuteSignedTransaction::new(self.tx, signer.into())
    }
}

#[derive(Debug, Clone)]
pub struct ConstructTransaction {
    pub tr: PrepopulateTransaction,
}

impl ConstructTransaction {
    pub fn new(signer_id: AccountId, receiver_id: AccountId) -> Self {
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

    pub fn with_signer(self, signer: Signer) -> ExecuteSignedTransaction<Self> {
        ExecuteSignedTransaction::new(self, signer.into())
    }
}

impl Transactionable for ConstructTransaction {
    type Handler = ();
    type Error = Infallible;

    fn prepopulated(&self) -> PrepopulateTransaction {
        PrepopulateTransaction {
            signer_id: self.tr.signer_id.clone(),
            receiver_id: self.tr.receiver_id.clone(),
            actions: self.tr.actions.clone(),
        }
    }

    fn validate_with_network(
        &self,
        _: &NetworkConfig,
        _query_response: Option<()>,
    ) -> Result<(), Infallible> {
        Ok(())
    }

    fn prequery(&self) -> Option<QueryBuilder<()>> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Transaction;

impl Transaction {
    pub fn construct(signer_id: AccountId, receiver_id: AccountId) -> ConstructTransaction {
        ConstructTransaction::new(signer_id, receiver_id)
    }

    pub fn sign_transaction(
        unsigned_tx: near_primitives::transaction::Transaction,
        signer: Signer,
    ) -> Result<ExecuteSignedTransaction<ConstructTransaction>, SignerError> {
        ConstructTransaction::new(unsigned_tx.signer_id, unsigned_tx.receiver_id)
            .add_actions(unsigned_tx.actions)
            .with_signer(signer)
            .presign_offline(
                unsigned_tx.public_key,
                unsigned_tx.block_hash,
                unsigned_tx.nonce,
            )
    }
}
