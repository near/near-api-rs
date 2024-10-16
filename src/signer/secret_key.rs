use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};
use tracing::{debug, instrument, trace};

use crate::{errors::SignerError, types::transactions::PrepopulateTransaction};

use super::SignerTrait;

const SECRET_KEY_SIGNER_TARGET: &str = "near_api::signer::secret_key";

#[derive(Debug, Clone)]
pub struct SecretKeySigner {
    secret_key: SecretKey,
    public_key: PublicKey,
}

#[async_trait::async_trait]
impl SignerTrait for SecretKeySigner {
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    fn tx_and_secret(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError> {
        debug!(target: SECRET_KEY_SIGNER_TARGET, "Creating transaction");
        let mut transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash,
        );
        *transaction.actions_mut() = tr.actions;

        trace!(target: SECRET_KEY_SIGNER_TARGET, "Transaction created, returning with secret key");
        Ok((transaction, self.secret_key.clone()))
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        Ok(self.public_key.clone())
    }
}

impl SecretKeySigner {
    pub fn new(secret_key: SecretKey) -> Self {
        let public_key = secret_key.public_key();
        Self {
            secret_key,
            public_key,
        }
    }
}
