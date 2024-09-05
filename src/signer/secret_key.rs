use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};

use crate::{errors::SignerError, types::transactions::PrepopulateTransaction};

use super::SignerTrait;

#[derive(Debug, Clone)]
pub struct SecretKeySigner {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl SignerTrait for SecretKeySigner {
    fn tx_and_secret(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError> {
        let mut transaction = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash,
        );
        *transaction.actions_mut() = tr.actions;
        Ok((transaction, self.secret_key.clone()))
    }

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
