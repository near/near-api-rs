use std::path::PathBuf;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{hash::CryptoHash, transaction::Transaction, types::Nonce};

use super::{AccountKeyPair, SignerTrait};
use crate::types::transactions::PrepopulateTransaction;

#[derive(Debug, Clone)]
pub struct AccessKeyFileSigner {
    keypair: AccountKeyPair,
}

impl AccessKeyFileSigner {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let keypair = AccountKeyPair::load_access_key_file(&path)?;

        Ok(Self { keypair })
    }
}

impl SignerTrait for AccessKeyFileSigner {
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
            self.keypair.private_key.to_owned(),
        ))
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
        Ok(self.keypair.public_key.clone())
    }
}
