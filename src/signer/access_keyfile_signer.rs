use std::path::PathBuf;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};

use super::{AccountKeyPair, SignerTrait};
use crate::{
    errors::{AccessKeyFileError, SignerError},
    types::transactions::PrepopulateTransaction,
};

#[derive(Debug, Clone)]
pub struct AccessKeyFileSigner {
    keypair: AccountKeyPair,
}

impl AccessKeyFileSigner {
    pub fn new(path: PathBuf) -> Result<Self, AccessKeyFileError> {
        let keypair = AccountKeyPair::load_access_key_file(&path)?;

        Ok(Self { keypair })
    }
}

#[async_trait::async_trait]
impl SignerTrait for AccessKeyFileSigner {
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

        Ok((transaction, self.keypair.private_key.to_owned()))
    }

    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        Ok(self.keypair.public_key.clone())
    }
}
