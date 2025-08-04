use near_api_types::{
    AccountId, BlockHeight, CryptoHash, Nonce, PublicKey, SecretKey, Signature,
    crypto::KeyType,
    transaction::{
        PrepopulateTransaction, SignedTransaction, Transaction, TransactionV0,
        delegate_action::{DelegateAction, NonDelegateAction, SignedDelegateAction},
    },
};
use slipped10::BIP32Path;
use tokio::sync::OnceCell;
use tracing::{debug, info, instrument, warn};

use crate::errors::{LedgerError, MetaSignError, SignerError};

use super::{NEP413Payload, SignerTrait};

const LEDGER_SIGNER_TARGET: &str = "near_api::signer::ledger";

#[derive(Debug, Clone)]
pub struct LedgerSigner {
    hd_path: BIP32Path,
    public_key: OnceCell<PublicKey>,
}

impl LedgerSigner {
    pub const fn new(hd_path: BIP32Path) -> Self {
        Self {
            hd_path,
            public_key: OnceCell::const_new(),
        }
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
    ) -> Result<SignedTransaction, SignerError> {
        debug!(target: LEDGER_SIGNER_TARGET, "Preparing unsigned transaction");
        let unsigned_tx = Transaction::V0(TransactionV0 {
            signer_id: tr.signer_id.clone(),
            public_key,
            receiver_id: tr.receiver_id,
            nonce,
            block_hash,
            actions: tr.actions,
        });
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
        let signature = Signature::from_parts(KeyType::ED25519, signature.as_ref())
            .map_err(|e| LedgerError::SignatureDeserializationError(e.to_string()))?;

        info!(target: LEDGER_SIGNER_TARGET, "Transaction signed successfully");
        Ok(SignedTransaction::new(signature, unsigned_tx))
    }

    #[instrument(skip(self, tr), fields(signer_id = %tr.signer_id, receiver_id = %tr.receiver_id))]
    async fn sign_meta(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        _block_hash: CryptoHash,
        max_block_height: BlockHeight,
    ) -> Result<SignedDelegateAction, MetaSignError> {
        debug!(target: LEDGER_SIGNER_TARGET, "Preparing delegate action");
        let actions = tr
            .actions
            .into_iter()
            .map(NonDelegateAction::try_from)
            .collect::<Result<_, _>>()
            .map_err(|_| MetaSignError::DelegateActionIsNotSupported)?;
        let delegate_action = DelegateAction {
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
            Signature::from_parts(KeyType::ED25519, signature.as_ref()).map_err(|e| {
                SignerError::LedgerError(LedgerError::SignatureDeserializationError(e.to_string()))
            })?;

        info!(target: LEDGER_SIGNER_TARGET, "Delegate action signed successfully");
        Ok(SignedDelegateAction {
            delegate_action,
            signature,
        })
    }

    #[instrument(skip(self), fields(signer_id = %_signer_id, receiver_id = %payload.recipient, message = %payload.message))]
    async fn sign_message_nep413(
        &self,
        _signer_id: AccountId,
        _public_key: PublicKey,
        payload: NEP413Payload,
    ) -> Result<Signature, SignerError> {
        info!(target: LEDGER_SIGNER_TARGET, "Signing NEP413 message with Ledger");
        let hd_path = self.hd_path.clone();
        let payload = payload.into();

        let signature: Vec<u8> = tokio::task::spawn_blocking(move || {
            let signature =
                near_ledger::sign_message_nep413(&payload, hd_path).map_err(LedgerError::from)?;

            Ok::<_, LedgerError>(signature)
        })
        .await
        .unwrap_or_else(|tokio_join_error| Err(LedgerError::from(tokio_join_error)))?;

        debug!(target: LEDGER_SIGNER_TARGET, "Creating Signature object for NEP413");
        let signature =
            Signature::from_parts(KeyType::ED25519, signature.as_ref()).map_err(|e| {
                SignerError::LedgerError(LedgerError::SignatureDeserializationError(e.to_string()))
            })?;

        Ok(signature)
    }

    async fn get_secret_key(
        &self,
        _signer_id: &AccountId,
        _public_key: &PublicKey,
    ) -> Result<SecretKey, SignerError> {
        warn!(target: LEDGER_SIGNER_TARGET, "Attempted to access secret key, which is not available for Ledger signer");
        Err(SignerError::SecretKeyIsNotAvailable)
    }

    #[instrument(skip(self))]
    fn get_public_key(&self) -> Result<PublicKey, SignerError> {
        if let Some(public_key) = self.public_key.get() {
            Ok(public_key.clone())
        } else {
            let public_key = near_ledger::get_wallet_id(self.hd_path.clone())
                .map_err(|_| SignerError::PublicKeyIsNotAvailable)?;
            let public_key = PublicKey::ED25519(near_api_types::crypto::public_key::ED25519PublicKey(
                *public_key.as_bytes(),
            ));
            self.public_key
                .set(public_key.clone())
                .map_err(LedgerError::from)?;
            Ok(public_key)
        }
    }
}
