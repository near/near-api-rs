use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};

use crate::transactions::PrepopulateTransaction;

use super::SignerTrait;

#[derive(Debug, Clone)]
pub struct SecretKeySigner {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl SignerTrait for SecretKeySigner {
    fn unsigned_tx(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        Ok((
            near_primitives::transaction::Transaction {
                public_key,
                block_hash,
                nonce,
                signer_id: tr.signer_id.clone(),
                receiver_id: tr.receiver_id.clone(),
                actions: tr.actions.clone(),
            },
            self.secret_key.clone(),
        ))
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
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
