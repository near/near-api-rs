use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    action::delegate::SignedDelegateAction, hash::CryptoHash, transaction::Transaction,
    types::Nonce,
};
use slipped10::BIP32Path;
use tracing::{debug, info, instrument, trace, warn};

use crate::{
    errors::{LedgerError, MetaSignError, SignerError},
    types::transactions::PrepopulateTransaction,
};

use super::SignerTrait;

const LEDGER_SIGNER_TARGET: &str = "near_api::signer::ledger";

#[derive(Debug, Clone)]
pub struct LedgerSigner {
    hd_path: BIP32Path,
}

impl LedgerSigner {
    pub const fn new(hd_path: BIP32Path) -> Self {
        Self { hd_path }
    }
}

#[async_trait::async_trait]
impl SignerTrait for LedgerSigner {
    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    async fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> Result<near_primitives::transaction::SignedTransaction, SignerError> {
        debug!(target: LEDGER_SIGNER_TARGET, "Preparing unsigned transaction");
        let mut unsigned_tx = Transaction::new_v0(
            tr.signer_id.clone(),
            public_key,
            tr.receiver_id,
            nonce,
            block_hash,
        );
        *unsigned_tx.actions_mut() = tr.actions;
        let unsigned_tx_bytes = borsh::to_vec(&unsigned_tx).map_err(LedgerError::from)?;
        let hd_path = self.hd_path.clone();

        info!(target: LEDGER_SIGNER_TARGET, "Signing transaction with Ledger");
        let signature = tokio::task::spawn_blocking(move || {
            let unsigned_tx_bytes = unsigned_tx_bytes;
            let signature = near_ledger::sign_transaction(&unsigned_tx_bytes, hd_path)
                .map_err(LedgerError::from)?;

            Ok::<_, LedgerError>(signature)
        })
        .await
        .map_err(LedgerError::from)?;

        let signature = signature?;

        debug!(target: LEDGER_SIGNER_TARGET, "Creating Signature object");
        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .map_err(LedgerError::from)?;

        info!(target: LEDGER_SIGNER_TARGET, "Transaction signed successfully");
        Ok(near_primitives::transaction::SignedTransaction::new(
            signature,
            unsigned_tx,
        ))
    }

    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    async fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        _block_hash: CryptoHash,
        max_block_height: near_primitives::types::BlockHeight,
    ) -> Result<near_primitives::action::delegate::SignedDelegateAction, MetaSignError> {
        debug!(target: LEDGER_SIGNER_TARGET, "Preparing delegate action");
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

        let delegate_action_bytes = borsh::to_vec(&delegate_action)
            .map_err(LedgerError::from)
            .map_err(SignerError::from)?;
        let hd_path = self.hd_path.clone();

        info!(target: LEDGER_SIGNER_TARGET, "Signing delegate action with Ledger");
        let signature = tokio::task::spawn_blocking(move || {
            let delegate_action_bytes = delegate_action_bytes;
            let signature =
                near_ledger::sign_message_nep366_delegate_action(&delegate_action_bytes, hd_path)
                    .map_err(LedgerError::from)?;

            Ok::<_, LedgerError>(signature)
        })
        .await
        .map_err(LedgerError::from)
        .map_err(SignerError::from)?;

        let signature = signature.map_err(SignerError::from)?;

        debug!(target: LEDGER_SIGNER_TARGET, "Creating Signature object for delegate action");
        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .map_err(LedgerError::from)
                .map_err(SignerError::from)?;

        info!(target: LEDGER_SIGNER_TARGET, "Delegate action signed successfully");
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
        warn!(target: LEDGER_SIGNER_TARGET, "Attempted to access secret key, which is not available for Ledger signer");
        Err(SignerError::SecretKeyIsNotAvailable)
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        let public_key = near_ledger::get_wallet_id(self.hd_path.clone())
            .map_err(|_| SignerError::PublicKeyIsNotAvailable)?;

        trace!(target: LEDGER_SIGNER_TARGET, "Public key retrieved successfully");
        Ok(near_crypto::PublicKey::ED25519(
            near_crypto::ED25519PublicKey::from(public_key.to_bytes()),
        ))
    }
}
