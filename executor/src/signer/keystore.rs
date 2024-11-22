use tracing::{debug, info, instrument, trace, warn};

use crate::{
    config::NetworkConfig,
    errors::{KeyStoreError, SignerError},
    query::{AccessKeyListHandler, QueryBuilder, RpcBuilder, SimpleQuery},
};
use near_types::{
    reference::Reference, transactions::PrepopulateTransaction, views::AccessKeyPermission,
    AccountId, CryptoHash, Nonce, PublicKey, SecretKey, Transaction,
};

use super::{AccountKeyPair, SignerTrait};

const KEYSTORE_SIGNER_TARGET: &str = "near_api::signer::keystore";

#[derive(Debug, Clone)]
pub struct KeystoreSigner {
    potential_pubkeys: Vec<PublicKey>,
}

#[async_trait::async_trait]
impl SignerTrait for KeystoreSigner {
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    fn tx_and_secret(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError> {
        debug!(target: KEYSTORE_SIGNER_TARGET, "Searching for matching public key");
        self.potential_pubkeys
            .iter()
            .find(|key| *key == &public_key)
            .ok_or(SignerError::PublicKeyIsNotAvailable)?;

        info!(target: KEYSTORE_SIGNER_TARGET, "Retrieving secret key");
        // TODO: fix this. Well the search is a bit suboptimal, but it's not a big deal for now
        let secret = Self::get_secret_key(&tr.signer_id, &public_key, "mainnet")
            .or_else(|_| Self::get_secret_key(&tr.signer_id, &public_key, "testnet"))
            .map_err(|_| SignerError::SecretKeyIsNotAvailable)?;

        debug!(target: KEYSTORE_SIGNER_TARGET, "Creating transaction");
        let mut transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash.into(),
        );
        *transaction.actions_mut() = tr.actions.into_iter().map(Into::into).collect();

        info!(target: KEYSTORE_SIGNER_TARGET, "Transaction and secret key prepared successfully");
        Ok((transaction, secret.private_key))
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        debug!(target: KEYSTORE_SIGNER_TARGET, "Retrieving first public key");
        self.potential_pubkeys
            .first()
            .cloned()
            .ok_or(SignerError::PublicKeyIsNotAvailable)
    }
}

impl KeystoreSigner {
    pub fn new_with_pubkey(pub_key: PublicKey) -> Self {
        debug!(target: KEYSTORE_SIGNER_TARGET, "Creating new KeystoreSigner with public key");
        Self {
            potential_pubkeys: vec![pub_key],
        }
    }

    #[instrument(skip(network), fields(account_id = %account_id, network_name = %network.network_name))]
    pub async fn search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> Result<Self, KeyStoreError> {
        info!(target: KEYSTORE_SIGNER_TARGET, "Searching for keys for account");
        let request = near_primitives::views::QueryRequest::ViewAccessKeyList {
            account_id: account_id.clone(),
        };
        let querier: QueryBuilder<AccessKeyListHandler> = RpcBuilder::new(
            SimpleQuery { request },
            Reference::Optimistic,
            Default::default(),
        );
        let account_keys = querier.fetch_from(network).await?;

        debug!(target: KEYSTORE_SIGNER_TARGET, "Filtering and collecting potential public keys");
        let potential_pubkeys: Vec<PublicKey> = account_keys
            .data
            .keys
            .into_iter()
            // TODO: support functional access keys
            .filter(|key| key.access_key.permission == AccessKeyPermission::FullAccess)
            .flat_map(|key| {
                Self::get_secret_key(&account_id, &key.public_key, &network.network_name)
                    .map(|keypair| keypair.public_key)
                    .ok()
            })
            .collect();

        info!(target: KEYSTORE_SIGNER_TARGET, "KeystoreSigner created with {} potential public keys", potential_pubkeys.len());
        Ok(Self { potential_pubkeys })
    }

    #[instrument(skip(public_key), fields(account_id = %account_id, network_name = %network_name))]
    fn get_secret_key(
        account_id: &AccountId,
        public_key: &PublicKey,
        network_name: &str,
    ) -> Result<AccountKeyPair, KeyStoreError> {
        trace!(target: KEYSTORE_SIGNER_TARGET, "Retrieving secret key from keyring");
        let service_name =
            std::borrow::Cow::Owned(format!("near-{}-{}", network_name, account_id.as_str()));

        let password =
            keyring::Entry::new(&service_name, &format!("{}:{}", account_id, public_key))?
                .get_password()?;

        debug!(target: KEYSTORE_SIGNER_TARGET, "Deserializing account key pair");
        Ok(serde_json::from_str(&password)?)
    }
}