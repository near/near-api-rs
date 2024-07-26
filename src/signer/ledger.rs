use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction, hash::CryptoHash, transaction::Transaction,
    types::Nonce,
};
use slipped10::BIP32Path;

use crate::{
    errors::{LedgerError, MetaSignError, SignerError},
    types::transactions::PrepopulateTransaction,
};

use super::SignerTrait;

// TODO: currently the ledger is blocking the thread as it's implemented synchronously.

#[derive(Debug, Clone)]
pub struct LedgerSigner {
    hd_path: BIP32Path,
}

impl LedgerSigner {
    pub fn new(hd_path: BIP32Path) -> Self {
        Self { hd_path }
    }
}

impl SignerTrait for LedgerSigner {
    fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<near_primitives::transaction::SignedTransaction, SignerError> {
        let unsigned_tx = near_primitives::transaction::Transaction {
            public_key,
            block_hash,
            nonce,
            signer_id: tr.signer_id.clone(),
            receiver_id: tr.receiver_id.clone(),
            actions: tr.actions.clone(),
        };

        let signature = near_ledger::sign_transaction(
            borsh::to_vec(&unsigned_tx).map_err(LedgerError::from)?,
            self.hd_path.clone(),
        )
        .map_err(LedgerError::from)?;

        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .map_err(LedgerError::from)?;

        Ok(near_primitives::transaction::SignedTransaction::new(
            signature,
            unsigned_tx,
        ))
    }

    fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        _block_hash: CryptoHash,
        max_block_height: near_primitives::types::BlockHeight,
    ) -> Result<near_primitives::action::delegate::SignedDelegateAction, MetaSignError> {
        let actions = tr
            .actions
            .into_iter()
            .map(near_primitives::action::delegate::NonDelegateAction::try_from)
            .collect::<Result<_, _>>()
            .map_err(|_| MetaSignError::DelegateActionIsNotSupported)?;
        let delegate_action = near_primitives::action::delegate::DelegateAction {
            sender_id: tr.signer_id,
            receiver_id: tr.receiver_id,
            actions,
            nonce,
            max_block_height,
            public_key,
        };

        let signature = near_ledger::sign_message_nep366_delegate_action(
            &delegate_action,
            self.hd_path.clone(),
        )
        .map_err(LedgerError::from)
        .map_err(SignerError::from)?;

        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .map_err(LedgerError::from)
                .map_err(SignerError::from)?;

        Ok(SignedDelegateAction {
            delegate_action,
            signature,
        })
    }

    fn tx_and_secret(
        &self,
        _tr: PrepopulateTransaction,
        _public_key: PublicKey,
        _nonce: Nonce,
        _block_hash: CryptoHash,
    ) -> Result<(Transaction, SecretKey), SignerError> {
        Err(SignerError::SecretKeyIsNotAvailable)
    }

    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        let public_key = near_ledger::get_wallet_id(self.hd_path.clone())
            .map_err(|_| SignerError::PublicKeyIsNotAvailable)?;
        Ok(near_crypto::PublicKey::ED25519(
            near_crypto::ED25519PublicKey::from(public_key.to_bytes()),
        ))
    }
}
