use futures::future::join_all;
use near_types::{AccessKeyPermission, AccountId, PublicKey, SecretKey};
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    config::NetworkConfig,
    errors::{KeyStoreError, SignerError},
};

use super::{AccountKeyPair, SignerTrait};

const KEYSTORE_SIGNER_TARGET: &str = "near_api::signer::keystore";

#[derive(Debug, Clone)]
pub struct KeystoreSigner {
    potential_pubkeys: Vec<PublicKey>,
}

#[async_trait::async_trait]
impl SignerTrait for KeystoreSigner {
    #[instrument(skip(self))]
    async fn get_secret_key(
        &self,
        signer_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<SecretKey, SignerError> {
        debug!(target: KEYSTORE_SIGNER_TARGET, "Searching for matching public key");
        self.potential_pubkeys
            .iter()
            .find(|key| *key == public_key)
            .ok_or(SignerError::PublicKeyIsNotAvailable)?;

        info!(target: KEYSTORE_SIGNER_TARGET, "Retrieving secret key");
        // TODO: fix this. Well the search is a bit suboptimal, but it's not a big deal for now
        let secret = if let Ok(secret) =
            Self::get_secret_key(signer_id, public_key.clone(), "mainnet").await
        {
            secret
        } else {
            Self::get_secret_key(signer_id, public_key.clone(), "testnet")
                .await
                .map_err(|_| SignerError::SecretKeyIsNotAvailable)?
        };

        info!(target: KEYSTORE_SIGNER_TARGET, "Secret key prepared successfully");
        Ok(secret.private_key)
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
        let account_keys = crate::account::Account(account_id.clone())
            .list_keys()
            .fetch_from(network)
            .await
            .map_err(KeyStoreError::QueryError)?;

        debug!(target: KEYSTORE_SIGNER_TARGET, "Filtering and collecting potential public keys");
        let potential_pubkeys = account_keys
            .data
            .iter()
            // TODO: support functional access keys
            .filter(|key| matches!(key.access_key.permission, AccessKeyPermission::FullAccess))
            .map(|key| key.public_key.clone())
            .map(|key| Self::get_secret_key(&account_id, key, &network.network_name));
        let potential_pubkeys: Vec<PublicKey> = join_all(potential_pubkeys)
            .await
            .into_iter()
            .flat_map(|result| result.map(|keypair| keypair.public_key).ok())
            .collect();

        info!(target: KEYSTORE_SIGNER_TARGET, "KeystoreSigner created with {} potential public keys", potential_pubkeys.len());
        Ok(Self { potential_pubkeys })
    }

    #[instrument(skip(public_key), fields(account_id = %account_id, network_name = %network_name))]
    async fn get_secret_key(
        account_id: &AccountId,
        public_key: PublicKey,
        network_name: &str,
    ) -> Result<AccountKeyPair, KeyStoreError> {
        trace!(target: KEYSTORE_SIGNER_TARGET, "Retrieving secret key from keyring");
        let service_name =
            std::borrow::Cow::Owned(format!("near-{}-{}", network_name, account_id.as_str()));
        let user = format!("{account_id}:{}", public_key);

        // This can be a blocking operation (for example, if the keyring is locked in the OS and user needs to unlock it),
        // so we need to spawn a new task to get the password
        let password = tokio::task::spawn_blocking(move || {
            let password = keyring::Entry::new(&service_name, &user)?.get_password()?;

            Ok::<_, KeyStoreError>(password)
        })
        .await
        .unwrap_or_else(|tokio_join_error| Err(KeyStoreError::from(tokio_join_error)))?;

        debug!(target: KEYSTORE_SIGNER_TARGET, "Deserializing account key pair");
        Ok(serde_json::from_str(&password)?)
    }
}
