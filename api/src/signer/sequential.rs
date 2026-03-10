use std::sync::Arc;

use futures::lock::Mutex;
use near_api_types::{
    AccountId, BlockHeight, CryptoHash, Nonce, PublicKey, Reference, TxExecutionStatus,
    transaction::{PrepopulateTransaction, SignedTransaction, result::TransactionResult},
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

    async fn get_sequential_nonce(&self, key: TransactionGroupKey) -> Arc<Mutex<u64>> {
        self.sequential_nonces
            .lock()
            .await
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(0)))
            .clone()
    }

    /// Fetches the transaction nonce and block hash associated to the access key. Internally
    /// caches the nonce as to not need to query for it every time, and ending up having to run
    /// into contention with others.
    ///
    /// Uses finalized block hash to avoid "Transaction Expired" errors when sending transactions
    /// to load-balanced RPC endpoints where different nodes may be at different chain heights.
    ///
    /// NOTE: This shouldn't be used during sequential sending
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
        let lock = self.get_sequential_nonce(key).await;
        let mut nonce = lock.lock().await;

        // It is important to fetch the nonce data after lock to get fresh block hash
        let (cached_nonce, block_hash, block_height) =
            Self::fetch_nonce_data(account_id, public_key, network).await?;

        *nonce = (*nonce).max(cached_nonce) + 1;

        Ok((*nonce, block_hash, block_height))
    }

    /// Signs and sends a transaction to the network.
    ///
    /// This method is used to sign and send a transaction to the network.
    /// Transactions of the same transaction group (network, account, public key)
    /// sent through this method will be sent sequentially
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
    /// Transactions of the same transaction group (network, account, public key)
    /// sent through this method will be sent sequentially
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
    pub(crate) async fn broadcast_tx(
        &self,
        account_id: impl Into<AccountId>,
        public_key: PublicKey,
        network: &NetworkConfig,
        transaction: PrepopulateTransaction,
        wait_until: TxExecutionStatus,
    ) -> Result<(SignedTransaction, TransactionResult), ExecuteTransactionError> {
        debug!(target: SIGNER_TARGET, "Broadcasting transaction");

        let account_id = account_id.into();

        // Locking until the transaction is sent
        let key = (network.network_name.clone(), account_id.clone(), public_key);
        let lock = self.get_sequential_nonce(key).await;
        let mut nonce = lock.lock().await;

        // It is important to fetch the nonce data after lock to get fresh block hash
        let (cached_nonce, block_hash, _) = Self::fetch_nonce_data(account_id, public_key, network)
            .await
            .map_err(MetaSignError::from)?;

        *nonce = (*nonce).max(cached_nonce) + 1;

        let signed = self
            .sign(transaction, public_key, *nonce, block_hash)
            .await?;

        let result =
            ExecuteSignedTransaction::send_impl(network, signed.clone(), wait_until).await?;

        Ok((signed, result))
    }

    #[allow(clippy::significant_drop_tightening)]
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

        // Locking until the transaction is sent
        let key = (network.network_name.clone(), account_id.clone(), public_key);
        let lock = self.get_sequential_nonce(key).await;
        let mut nonce = lock.lock().await;

        // It is important to fetch the nonce data after lock to get fresh block hash
        let (cached_nonce, block_hash, block_height) =
            Self::fetch_nonce_data(account_id, public_key, network)
                .await
                .map_err(MetaSignError::from)?;

        *nonce = (*nonce).max(cached_nonce) + 1;

        let signed = self
            .sign_meta(
                transaction,
                public_key,
                *nonce,
                block_hash,
                block_height + tx_live_for,
            )
            .await?;

        ExecuteMetaTransaction::send_impl(network, signed).await
    }
}
