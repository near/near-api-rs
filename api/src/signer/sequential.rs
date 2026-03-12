use std::time::Duration;

use near_api_types::{
    AccountId, BlockHeight, CryptoHash, Nonce, PublicKey, Reference, TxExecutionStatus,
    transaction::{PrepopulateTransaction, result::TransactionResult},
};

use near_openapi_client::types::RpcTransactionError;
use tokio::time::sleep;
use tracing::{debug, error, instrument, warn};

use crate::{
    Signer,
    advanced::{ExecuteMetaTransaction, ExecuteSignedTransaction, TxExecutionResult},
    config::NetworkConfig,
    errors::{
        ExecuteMetaTransactionsError, ExecuteTransactionError, MetaSignError, RetryError,
        SendRequestError, SignerError,
    },
    signer::SIGNER_TARGET,
};

impl Signer {
    async fn fetch_nonce_data(
        account_id: AccountId,
        public_key: PublicKey,
        network: &NetworkConfig,
    ) -> Result<(Nonce, CryptoHash, BlockHeight), SignerError> {
        debug!(target: SIGNER_TARGET, "Fetching latest nonce");

        let nonce_data = crate::account::Account(account_id.clone())
            .access_key(public_key)
            .at(Reference::Final)
            .fetch_from(network)
            .await
            .map_err(|e| SignerError::FetchNonceError(Box::new(e)))?;

        Ok((
            nonce_data.data.nonce.0,
            nonce_data.block_hash,
            nonce_data.block_height,
        ))
    }

    /// Fetches the transaction nonce and block hash associated to the access key. Internally
    /// caches the nonce as to not need to query for it every time, and ending up having to run
    /// into contention with others.
    ///
    /// Uses finalized block hash to avoid "Transaction Expired" errors when sending transactions
    /// to load-balanced RPC endpoints where different nodes may be at different chain heights.
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(skip(self, network))]
    pub async fn fetch_tx_nonce(
        &self,
        account_id: AccountId,
        public_key: PublicKey,
        network: &NetworkConfig,
    ) -> Result<(Nonce, CryptoHash, BlockHeight), SignerError> {
        debug!(target: SIGNER_TARGET, "Fetching transaction nonce");

        let key = (network.network_name.clone(), account_id.clone(), public_key);

        let (fetched_nonce, block_hash, block_height) =
            Self::fetch_nonce_data(account_id, public_key, network).await?;

        let nonce = {
            let mut nonce_cache = self.nonce_cache.lock().await;
            let nonce = nonce_cache.entry(key).or_default();

            *nonce = (*nonce).max(fetched_nonce) + 1;
            *nonce
        };

        Ok((nonce, block_hash, block_height))
    }

    /// Signs and sends a transaction to the network.
    ///
    /// Concurrent broadcasting of transactions of the same transaction group
    /// (network, account, public key) can cause nonce conflicts
    /// (`InvalidTransaction` errors), so this method retries with a fresh nonce
    /// up to `MAX_NONCE_ATTEMPTS` times before giving up.
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

        self.sign_and_send_with_retry(account_id, public_key, network, transaction, wait_until)
            .await
    }

    async fn sign_and_send_with_retry(
        &self,
        account_id: AccountId,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> TxExecutionResult {
        const MAX_NONCE_RETRIES: u32 = 3;

        let mut last_error = None;

        for attempt in 0..MAX_NONCE_RETRIES {
            debug!(
                target: SIGNER_TARGET,
                account_id = %account_id,
                attempt = attempt + 1,
                max_attempts = MAX_NONCE_RETRIES,
                "Attempting to broadcast transaction"
            );

            match self
                .broadcast_tx(
                    account_id.clone(),
                    public_key,
                    network,
                    transaction.clone(),
                    wait_until,
                )
                .await
            {
                Ok(result) => {
                    debug!(
                        target: SIGNER_TARGET,
                        account_id = %account_id,
                        attempt = attempt + 1,
                        "Transaction broadcast successful"
                    );

                    return Ok(result);
                }

                Err(err)
                    if Self::is_retryable_nonce_error(&err) && attempt + 1 < MAX_NONCE_RETRIES =>
                {
                    warn!(
                        target: SIGNER_TARGET,
                        account_id = %account_id,
                        attempt = attempt + 1,
                        max_attempts = MAX_NONCE_RETRIES,
                        error = ?err,
                        "Invalid transaction detected, retrying after delay"
                    );

                    last_error = Some(err);

                    // exponential backoff
                    let delay = Self::calculate_retry_delay(attempt);
                    sleep(delay).await;
                }

                Err(err) => {
                    error!(
                        target: SIGNER_TARGET,
                        account_id = %account_id,
                        attempt = attempt + 1,
                        error = ?err,
                        "Transaction broadcast failed"
                    );
                    return Err(err);
                }
            }
        }

        error!(
            target: SIGNER_TARGET,
            account_id = %account_id,
            max_attempts = MAX_NONCE_RETRIES,
            "All retry attempts exhausted"
        );

        Err(last_error.unwrap())
    }

    const fn is_retryable_nonce_error(error: &ExecuteTransactionError) -> bool {
        // TODO: check tx nonce error after fix in near openapi types
        matches!(
            error,
            ExecuteTransactionError::TransactionError(RetryError::Critical(
                SendRequestError::ServerError(RpcTransactionError::InvalidTransaction(_))
            ))
        )
    }

    fn calculate_retry_delay(attempt: u32) -> Duration {
        const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);

        INITIAL_RETRY_DELAY * 2u32.pow(attempt)
    }

    /// Signs and sends a meta transaction to the relayer.
    ///
    /// This method is used to sign and send a meta transaction to the relayer.
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

        self.broadcast_meta_tx(account_id, public_key, network, transaction, tx_live_for)
            .await
    }

    #[allow(clippy::significant_drop_tightening)]
    #[instrument(skip(self, account_id, network))]
    async fn broadcast_tx(
        &self,
        account_id: impl Into<AccountId>,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> Result<TransactionResult, ExecuteTransactionError> {
        debug!(target: SIGNER_TARGET, "Broadcasting transaction");
        let account_id = account_id.into();

        let (fetched_nonce, block_hash, _) =
            self.fetch_tx_nonce(account_id, public_key, network).await?;

        let signed = self
            .sign(transaction, public_key, fetched_nonce, block_hash)
            .await?;

        ExecuteSignedTransaction::send_impl(network, signed, wait_until).await
    }

    #[allow(clippy::significant_drop_tightening)]
    #[instrument(skip(self, account_id, network))]
    async fn broadcast_meta_tx(
        &self,
        account_id: impl Into<AccountId>,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        tx_live_for: BlockHeight,
    ) -> Result<reqwest::Response, ExecuteMetaTransactionsError> {
        debug!(target: SIGNER_TARGET, "Broadcasting meta transaction");
        let account_id = account_id.into();

        let (fetched_nonce, block_hash, block_height) = self
            .fetch_tx_nonce(account_id, public_key, network)
            .await
            .map_err(MetaSignError::from)?;

        let signed = self
            .sign_meta(
                transaction,
                public_key,
                fetched_nonce,
                block_hash,
                block_height + tx_live_for,
            )
            .await?;

        ExecuteMetaTransaction::send_impl(network, signed).await
    }
}
