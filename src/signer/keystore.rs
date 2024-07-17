use anyhow::Context;
use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    hash::CryptoHash,
    transaction::Transaction,
    types::{AccountId, Nonce},
    views::AccessKeyPermissionView,
};

use crate::{config::NetworkConfig, types::transactions::PrepopulateTransaction};

use super::{AccountKeyPair, SignerTrait};

#[derive(Debug, Clone)]
pub struct KeystoreSigner {
    potential_pubkeys: Vec<PublicKey>,
}

impl SignerTrait for KeystoreSigner {
    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        self.potential_pubkeys
            .iter()
            .find(|key| *key == &public_key)
            .context("Public key not found in keystore")?;

        // TODO: fix this. Well the search is a bit suboptimal, but it's not a big deal for now
        let secret = Self::get_secret_key(&tr.signer_id, &public_key, "mainnet")
            .or(Self::get_secret_key(&tr.signer_id, &public_key, "testnet"))
            .context("Secret key not found in keystore")?;

        Ok((
            near_primitives::transaction::Transaction {
                public_key,
                block_hash,
                nonce,
                signer_id: tr.signer_id.clone(),
                receiver_id: tr.receiver_id.clone(),
                actions: tr.actions.clone(),
            },
            secret.private_key,
        ))
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
        self.potential_pubkeys
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No public keys found in keystore"))
    }
}

impl KeystoreSigner {
    pub fn new_with_pubkey(pub_key: PublicKey) -> Self {
        Self {
            potential_pubkeys: vec![pub_key],
        }
    }

    pub async fn search_for_keys(
        account_id: AccountId,
        network: &NetworkConfig,
    ) -> anyhow::Result<Self> {
        let account_keys = crate::account::Account(account_id.clone())
            .list_keys()
            .fetch_from(network)
            .await?;

        let potential_pubkeys = account_keys
            .keys
            .into_iter()
            // TODO: support functional access keys
            .filter(|key| key.access_key.permission == AccessKeyPermissionView::FullAccess)
            .flat_map(|key| {
                Self::get_secret_key(&account_id, &key.public_key, &network.network_name)?;
                anyhow::Ok(key.public_key)
            })
            .collect();

        Ok(Self { potential_pubkeys })
    }

    fn get_secret_key(
        account_id: &AccountId,
        public_key: &PublicKey,
        network_name: &str,
    ) -> anyhow::Result<AccountKeyPair> {
        let service_name =
            std::borrow::Cow::Owned(format!("near-{}-{}", network_name, account_id.as_str()));

        let password =
            keyring::Entry::new(&service_name, &format!("{}:{}", account_id, public_key))?
                .get_password()?;

        Ok(serde_json::from_str(&password)?)
    }
}
