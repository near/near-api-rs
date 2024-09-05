use std::collections::HashMap;

use futures::future::join_all;
use near_primitives::views::FinalExecutionOutcomeView;

use crate::{
    common::send::{ExecuteSignedTransaction, Transactionable},
    errors::MultiTransactionError,
    signer::Signer,
    types::non_empty_vector::NonEmptyVec,
    NetworkConfig,
};

#[derive(Default)]
pub struct MultiTransactions {
    transactions: Vec<Box<dyn Transactionable>>,
    same_signer_concurrent: bool,
}

impl MultiTransactions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_transaction<T: Transactionable + 'static>(mut self, transaction: T) -> Self {
        self.transactions.push(Box::new(transaction));
        self
    }

    pub fn with_same_signer_concurrent(mut self, same_signer_concurrent: bool) -> Self {
        self.same_signer_concurrent = same_signer_concurrent;
        self
    }

    /// The list of sign keys that would be used to create paralel tx.
    /// if `same_signer_concurrent` is true, the transactions will be sent concurrently for same signer.
    /// Though, if the transaction with larger nonce arrives first, smaller nonce transactions will fail.
    pub fn with_signers(
        self,
        signers: Vec<Signer>,
    ) -> Result<SendSignedMultiTransactions, MultiTransactionError> {
        let signers = NonEmptyVec::new(signers)?;
        Self::validate_keys(&signers)?;

        let mut txs_per_signer: HashMap<usize, Vec<Box<dyn Transactionable + 'static>>> =
            HashMap::new();
        let signers_len = signers.inner().len();
        for (i, transaction) in self.transactions.into_iter().enumerate() {
            let signer_id = i % signers_len;
            txs_per_signer
                .entry(signer_id)
                .or_default()
                .push(transaction);
        }

        let execute_signed_transaction =
            signers
                .into_inner()
                .into_iter()
                .enumerate()
                .flat_map(|(i, signer)| {
                    let txs = txs_per_signer.remove(&i)?;
                    Some(
                        ExecuteSignedTransaction::new_multi(
                            NonEmptyVec::new(txs).expect("Expected non-empty vector"),
                            signer.into(),
                        )
                        .send_concurrent(self.same_signer_concurrent),
                    )
                });

        Ok(SendSignedMultiTransactions::new(
            execute_signed_transaction.collect(),
        ))
    }

    /// Runs the transactions with single signer and incremental nonce
    /// Please note that the transactions might be failed if the tx with larger nonce arrives first.
    pub fn with_signer(
        self,
        signer: Signer,
    ) -> Result<SendSignedMultiTransactions, MultiTransactionError> {
        self.with_signers(vec![signer])
    }

    fn validate_keys(signers: &NonEmptyVec<Signer>) -> Result<(), MultiTransactionError> {
        // It's fine to use Vec as on small amount of keys, array performs better
        let mut keys = Vec::default();

        for signer in signers.inner() {
            let public_key = signer.as_signer().get_public_key()?;
            if keys.contains(&public_key) {
                return Err(MultiTransactionError::DuplicateSigner);
            }
            keys.push(public_key);
        }

        Ok(())
    }
}

pub struct SendSignedMultiTransactions {
    transactions: Vec<ExecuteSignedTransaction>,
}

impl SendSignedMultiTransactions {
    pub fn new(transactions: Vec<ExecuteSignedTransaction>) -> Self {
        Self { transactions }
    }

    pub async fn send_to(
        self,
        network: &NetworkConfig,
    ) -> Result<Vec<FinalExecutionOutcomeView>, MultiTransactionError> {
        let data: Vec<_> = join_all(
            self.transactions
                .into_iter()
                .map(|tx| async { tx.send_to(network).await }),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|e| e.into_inner())
        .collect();

        Ok(data)
    }

    pub async fn send_to_mainnet(
        self,
    ) -> Result<Vec<FinalExecutionOutcomeView>, MultiTransactionError> {
        self.send_to(&NetworkConfig::mainnet()).await
    }

    pub async fn send_to_testnet(
        self,
    ) -> Result<Vec<FinalExecutionOutcomeView>, MultiTransactionError> {
        self.send_to(&NetworkConfig::testnet()).await
    }
}
