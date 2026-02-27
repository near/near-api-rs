use std::sync::Arc;

use near_api_types::{
    AccountId, BlockHeight, PublicKey, TxExecutionStatus,
    transaction::{PrepopulateTransaction, SignedTransaction, result::ExecutionFinalResult},
};

use near_openapi_client::types::RpcTransactionStatusRequest;
use tracing::{debug, instrument};

use crate::{
    Signer,
    advanced::{ExecuteMetaTransaction, ExecuteSignedTransaction, TxExecutionResult},
    config::NetworkConfig,
    errors::{ExecuteMetaTransactionsError, ExecuteTransactionError, MetaSignError, SignerError},
    signer::{SIGNER_TARGET, TransactionGroupKey},
};

impl Signer {
    fn get_sequential_lock(&self, key: TransactionGroupKey) -> Arc<tokio::sync::Mutex<()>> {
        self.sequential_locks
            .entry(key)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Signs and sends a transaction to the network.
    ///
    /// This method is used to sign and send a transaction to the network.
    /// It will use the sequential send mode if it is enabled.
    /// Otherwise, it will send the transaction non-sequentially.
    #[instrument(skip(self, network, transaction, account_id))]
    pub async fn sign_and_send(
        &self,
        account_id: impl Into<AccountId>,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> TxExecutionResult {
        let account_id = account_id.into();
        let public_key = self
            .get_public_key()
            .await
            .map_err(SignerError::PublicKeyError)?;

        let sequential = self
            .sequential_mode
            .load(std::sync::atomic::Ordering::SeqCst);

        if !sequential {
            let (_, result) = self
                .broadcast_tx(account_id, public_key, network, transaction, wait_until)
                .await?;
            return Ok(result);
        }

        let key = (account_id.clone(), public_key, network.network_name.clone());
        let lock = self.get_sequential_lock(key);
        let _guard = lock.lock().await;

        let (signed, result) = self
            .broadcast_tx(
                account_id,
                public_key,
                network,
                transaction,
                TxExecutionStatus::Included,
            )
            .await?;

        match wait_until {
            TxExecutionStatus::Included => Ok(result),
            _ => {
                ExecuteSignedTransaction::fetch_tx(
                    network,
                    RpcTransactionStatusRequest::Variant0 {
                        signed_tx_base64: signed.into(),
                        wait_until,
                    },
                )
                .await
            }
        }
    }

    /// Signs and sends a meta transaction to the relayer.
    ///
    /// This method is used to sign and send a meta transaction to the relayer.
    /// It will use the sequential send mode if it is enabled.
    /// Otherwise, it will send the transaction non-sequentially.
    #[instrument(skip(self, network, transaction, account_id))]
    pub async fn sign_and_send_meta(
        &self,
        account_id: impl Into<AccountId>,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        tx_live_for: BlockHeight,
    ) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        let account_id = account_id.into();
        let public_key = self
            .get_public_key()
            .await
            .map_err(SignerError::PublicKeyError)
            .map_err(MetaSignError::from)?;

        let sequential = self
            .sequential_mode
            .load(std::sync::atomic::Ordering::SeqCst);

        if !sequential {
            return self
                .broadcast_meta_tx(account_id, public_key, network, transaction, tx_live_for)
                .await;
        }

        let key = (account_id.clone(), public_key, network.network_name.clone());
        let lock = self.get_sequential_lock(key);
        let _guard = lock.lock().await;

        self.broadcast_meta_tx(account_id, public_key, network, transaction, tx_live_for)
            .await
    }

    #[instrument(skip(self, account_id, network))]
    pub(crate) async fn broadcast_tx(
        &self,
        account_id: impl Into<AccountId>,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> Result<(SignedTransaction, ExecutionFinalResult), ExecuteTransactionError> {
        debug!(target: SIGNER_TARGET, "Broadcasting transaction");

        let account_id = account_id.into();

        let (nonce, block_hash, _) = self.fetch_tx_nonce(account_id, public_key, network).await?;

        let signed = self
            .sign(transaction, public_key, nonce, block_hash)
            .await?;

        let result =
            ExecuteSignedTransaction::send_impl(network, signed.clone(), wait_until).await?;

        Ok((signed, result))
    }

    #[instrument(skip(self, account_id, network))]
    pub(crate) async fn broadcast_meta_tx(
        &self,
        account_id: impl Into<AccountId>,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        tx_live_for: BlockHeight,
    ) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        debug!(target: SIGNER_TARGET, "Broadcasting meta transaction");
        let account_id = account_id.into();

        let (nonce, block_hash, block_height) = self
            .fetch_tx_nonce(account_id, public_key, network)
            .await
            .map_err(MetaSignError::from)?;

        let signed = self
            .sign_meta(
                transaction,
                public_key,
                nonce,
                block_hash,
                block_height + tx_live_for,
            )
            .await?;

        ExecuteMetaTransaction::send_impl(network, signed).await
    }
}
