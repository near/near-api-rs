use std::path::PathBuf;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{transaction::Transaction, types::Nonce};
use tracing::{debug, instrument, trace};

use super::{AccountKeyPair, SignerTrait};
use crate::{
    errors::{AccessKeyFileError, SignerError},
    types::{transactions::PrepopulateTransaction, CryptoHash},
};

const ACCESS_KEYFILE_SIGNER_TARGET: &str = "near_api::signer::access_keyfile";

#[derive(Debug, Clone)]
pub struct AccessKeyFileSigner {
    keypair: AccountKeyPair,
}

impl AccessKeyFileSigner {
    #[instrument(skip(path), fields(path = %path.display()))]
    pub fn new(path: PathBuf) -> Result<Self, AccessKeyFileError> {
        let keypair = AccountKeyPair::load_access_key_file(&path)?;
        debug!(target: ACCESS_KEYFILE_SIGNER_TARGET, "Access key file loaded successfully");

        Ok(Self { keypair })
    }
}

#[async_trait::async_trait]
impl SignerTrait for AccessKeyFileSigner {
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    fn tx_and_secret(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError> {
        debug!(target: ACCESS_KEYFILE_SIGNER_TARGET, "Creating transaction");
        let mut transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash.into(),
        );
        *transaction.actions_mut() = tr.actions;

        trace!(target: ACCESS_KEYFILE_SIGNER_TARGET, "Transaction created, returning with secret key");
        Ok((transaction, self.keypair.private_key.to_owned()))
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        debug!(target: ACCESS_KEYFILE_SIGNER_TARGET, "Retrieving public key");
        Ok(self.keypair.public_key.clone())
    }
}
