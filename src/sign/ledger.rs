use anyhow::{bail, Context};
use near_crypto::{PublicKey, SecretKey, Signature};
use near_ledger::NEARLedgerError;
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    signable_message::{SignableMessage, SignableMessageType},
    transaction::Transaction,
    types::Nonce,
};
use serde::Deserialize;
use slipped10::BIP32Path;

use crate::transactions::PrepopulateTransaction;

use super::SignerTrait;

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

    pub fn sign_borsh(&self, data: Vec<u8>) -> anyhow::Result<Signature> {
        let signature = match near_ledger::sign_transaction(data, self.hd_path.clone()) {
            Ok(signature) => {
                near_crypto::Signature::from_parts(near_crypto::KeyType::ED25519, &signature)
                    .context("Signature is not expected to fail on deserialization")?
            }
            Err(NEARLedgerError::APDUExchangeError(msg)) if msg.contains(SW_BUFFER_OVERFLOW) => {
                bail!(ERR_OVERFLOW_MEMO);
            }
            Err(near_ledger_error) => {
                bail!(
                    "Error occurred while signing the transaction: {:?}",
                    near_ledger_error
                );
            }
        };
        Ok(signature)
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

        let signature = self.sign_borsh(
            borsh::to_vec(&unsigned_tx)
                .context("Unexpected to fail serialization of the unsigned tx")?,
        )?;

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

        let signable = SignableMessage::new(&delegate_action, SignableMessageType::DelegateAction);
        let borsh_data = borsh::to_vec(&signable)
            .context("Delegate action is not expected to fail on serialization")?;
        let signature = self.sign_borsh(borsh_data)?;
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

#[derive(Debug, Deserialize)]
pub struct AccountKeyPair {
    pub public_key: near_crypto::PublicKey,
    pub private_key: near_crypto::SecretKey,
}
