use anyhow::{bail, Context};
use near_crypto::{PublicKey, SecretKey};
use near_ledger::NEARLedgerError;
use near_primitives::{
    action::delegate::SignedDelegateAction, hash::CryptoHash, transaction::Transaction,
    types::Nonce,
};
use slipped10::BIP32Path;

use crate::types::transactions::PrepopulateTransaction;

use super::SignerTrait;

// TODO: currently the ledger is blocking the thread as it's implemented synchronously.

const SW_BUFFER_OVERFLOW: &str = "0x6990";
const ERR_OVERFLOW_MEMO: &str = "Buffer overflow on Ledger device occured. \
Transaction is too large for signature. \
This is resolved in https://github.com/dj8yfo/app-near-rs . \
The status is tracked in `About` section.";

#[derive(Debug, Clone)]
pub struct LedgerSigner {
    hd_path: BIP32Path,
}

impl LedgerSigner {
    pub fn new(hd_path: BIP32Path) -> Self {
        Self { hd_path }
    }
}

fn map_ledger_err(ledger_error: NEARLedgerError) -> anyhow::Error {
    match ledger_error {
        NEARLedgerError::APDUExchangeError(msg) if msg.contains(SW_BUFFER_OVERFLOW) => {
            anyhow::anyhow!(ERR_OVERFLOW_MEMO)
        }
        near_ledger_error => {
            anyhow::anyhow!(
                "Error occurred while signing the transaction: {:?}",
                near_ledger_error
            )
        }
    }
}

impl SignerTrait for LedgerSigner {
    fn sign(
        &self,
        tr: PrepopulateTransaction,
        public_key: PublicKey,
        nonce: Nonce,
        block_hash: CryptoHash,
    ) -> anyhow::Result<near_primitives::transaction::SignedTransaction> {
        let unsigned_tx = near_primitives::transaction::Transaction {
            public_key,
            block_hash,
            nonce,
            signer_id: tr.signer_id.clone(),
            receiver_id: tr.receiver_id.clone(),
            actions: tr.actions.clone(),
        };

        let signature = near_ledger::sign_transaction(
            borsh::to_vec(&unsigned_tx)
                .context("Unexpected to fail serialization of the unsigned tx")?,
            self.hd_path.clone(),
        )
        .map_err(map_ledger_err)?;

        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .context("Signature is not expected to fail on deserialization")?;

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
    ) -> anyhow::Result<near_primitives::action::delegate::SignedDelegateAction> {
        let actions = tr
            .actions
            .into_iter()
            .map(near_primitives::action::delegate::NonDelegateAction::try_from)
            .collect::<Result<_, _>>()
            .map_err(|_| anyhow::anyhow!("Delegate action can't contain delegate action"))?;
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
        .map_err(map_ledger_err)?;

        let signature =
            near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                .context("Signature is not expected to fail on deserialization")?;

        Ok(SignedDelegateAction {
            delegate_action,
            signature,
        })
    }

    fn unsigned_tx(
        &self,
        _tr: PrepopulateTransaction,
        _public_key: PublicKey,
        _nonce: Nonce,
        _block_hash: CryptoHash,
    ) -> anyhow::Result<(Transaction, SecretKey)> {
        bail!("LedgerSigner doesn't support unsigned_tx")
    }

    fn get_public_key(&self) -> anyhow::Result<PublicKey> {
        let public_key =
            near_ledger::get_wallet_id(self.hd_path.clone()).map_err(|near_ledger_error| {
                anyhow::anyhow!(
                    "An error occurred while trying to get PublicKey from Ledger device: {:?}",
                    near_ledger_error
                )
            })?;
        Ok(near_crypto::PublicKey::ED25519(
            near_crypto::ED25519PublicKey::from(public_key.to_bytes()),
        ))
    }
}
