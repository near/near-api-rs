use std::str::FromStr;

use near_crypto::{PublicKey, SecretKey};
use near_primitives::{action::delegate::SignedDelegateAction, transaction::Transaction};
use slipped10::BIP32Path;

use crate::{
    config::NetworkConfig,
    send::{SendMetaTransaction, SendSignedTransaction},
    transactions::PrepopulateTransaction,
};

#[derive(Debug, Clone)]
pub struct SignSeedPhrase {
    pub tr: PrepopulateTransaction,
    pub master_seed_phrase: String,
    pub hd_path: BIP32Path,
}

impl SignSeedPhrase {
    pub fn new(master_seed_phrase: String, tr: PrepopulateTransaction) -> Self {
        Self {
            tr,
            master_seed_phrase,
            hd_path: BIP32Path::from_str("m/44'/397'/0'").expect("Valid HD path"),
        }
    }

    pub fn hd_path(mut self, hd_path: BIP32Path) -> Self {
        self.hd_path = hd_path;
        self
    }

    pub fn sign_offline(self) -> anyhow::Result<SendSignedTransaction> {
        let (unsigned_transaction, signer_secret_key) = self.unsigned_tx()?;
        let signature = signer_secret_key.sign(unsigned_transaction.get_hash_and_size().0.as_ref());

        let signed_transaction =
            near_primitives::transaction::SignedTransaction::new(signature, unsigned_transaction);

        Ok(SendSignedTransaction { signed_transaction })
    }

    pub async fn sign_for(
        mut self,
        network: &NetworkConfig,
    ) -> anyhow::Result<SendSignedTransaction> {
        self.update_network_data(network).await?;
        self.sign_offline()
    }

    pub async fn sign_for_mainnet(self) -> anyhow::Result<SendSignedTransaction> {
        let network = NetworkConfig::mainnet();
        self.sign_for(&network).await
    }

    pub async fn sign_for_testnet(self) -> anyhow::Result<SendSignedTransaction> {
        let network = NetworkConfig::testnet();
        self.sign_for(&network).await
    }

    pub fn sign_offline_meta(self) -> anyhow::Result<SendMetaTransaction> {
        let (unsigned_transaction, signer_secret_key) = self.unsigned_tx()?;
        let max_block_height = self.tr.block_height.unwrap()
            + self
                .tr
                .meta_transaction_valid_for
                .unwrap_or(super::META_TRANSACTION_VALID_FOR_DEFAULT);

        let signed_delegate_action =
            get_signed_delegate_action(unsigned_transaction, signer_secret_key, max_block_height)?;

        Ok(SendMetaTransaction {
            signed_delegate_action,
        })
    }

    pub async fn sign_meta_for(
        mut self,
        network: &NetworkConfig,
    ) -> anyhow::Result<SendMetaTransaction> {
        self.update_network_data(network).await?;
        self.sign_offline_meta()
    }

    pub async fn sign_meta_for_mainnet(self) -> anyhow::Result<SendMetaTransaction> {
        let network = NetworkConfig::mainnet();
        self.sign_meta_for(&network).await
    }

    pub async fn sign_meta_for_testnet(self) -> anyhow::Result<SendMetaTransaction> {
        let network = NetworkConfig::testnet();
        self.sign_meta_for(&network).await
    }

    async fn update_network_data(&mut self, network: &NetworkConfig) -> anyhow::Result<()> {
        let key_pair_properties = get_key_pair_properties_from_seed_phrase(
            self.hd_path.clone(),
            self.master_seed_phrase.clone(),
        )?;

        let signer_public_key =
            near_crypto::PublicKey::from_str(&key_pair_properties.public_key_str)?;

        let response = crate::account::Account(self.tr.signer_id.clone())
            .access_key(signer_public_key.clone())
            .fetch_from(network)
            .await?;

        self.tr.nonce = Some(response.data.nonce + 1);
        self.tr.block_hash = Some(response.block_hash);
        self.tr.block_height = Some(response.block_height);
        Ok(())
    }

    fn unsigned_tx(&self) -> anyhow::Result<(Transaction, SecretKey)> {
        let key_pair_properties = get_key_pair_properties_from_seed_phrase(
            self.hd_path.clone(),
            self.master_seed_phrase.clone(),
        )?;

        let signer_secret_key: SecretKey =
            SecretKey::from_str(&key_pair_properties.secret_keypair_str)?;
        let signer_public_key = PublicKey::from_str(&key_pair_properties.public_key_str)?;

        if self.tr.nonce.is_none() || self.tr.block_hash.is_none() || self.tr.block_height.is_none()
        {
            return Err(anyhow::anyhow!(
                "Nonce, block hash, and block height must be set"
            ));
        }

        Ok((
            near_primitives::transaction::Transaction {
                public_key: signer_public_key.clone(),
                block_hash: self.tr.block_hash.unwrap(),
                nonce: self.tr.nonce.unwrap(),
                signer_id: self.tr.signer_id.clone(),
                receiver_id: self.tr.receiver_id.clone(),
                actions: self.tr.actions.clone(),
            },
            signer_secret_key,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct KeyPairProperties {
    pub seed_phrase_hd_path: BIP32Path,
    pub master_seed_phrase: String,
    pub implicit_account_id: near_primitives::types::AccountId,
    pub public_key_str: String,
    pub secret_keypair_str: String,
}

pub fn get_key_pair_properties_from_seed_phrase(
    seed_phrase_hd_path: BIP32Path,
    master_seed_phrase: String,
) -> anyhow::Result<KeyPairProperties> {
    let master_seed = bip39::Mnemonic::parse(&master_seed_phrase)?.to_seed("");
    let derived_private_key = slipped10::derive_key_from_path(
        &master_seed,
        slipped10::Curve::Ed25519,
        &seed_phrase_hd_path,
    )
    .map_err(|err| anyhow::anyhow!("Failed to derive a key from the master key: {}", err))?;

    let signing_key = ed25519_dalek::SigningKey::from_bytes(&derived_private_key.key);

    let public_key = signing_key.verifying_key();
    let implicit_account_id = near_primitives::types::AccountId::try_from(hex::encode(public_key))?;
    let public_key_str = format!("ed25519:{}", bs58::encode(&public_key).into_string());
    let secret_keypair_str = format!(
        "ed25519:{}",
        bs58::encode(signing_key.to_keypair_bytes()).into_string()
    );
    let key_pair_properties: KeyPairProperties = KeyPairProperties {
        seed_phrase_hd_path,
        master_seed_phrase,
        implicit_account_id,
        public_key_str,
        secret_keypair_str,
    };
    Ok(key_pair_properties)
}

pub fn get_signed_delegate_action(
    unsigned_transaction: Transaction,
    private_key: SecretKey,
    max_block_height: u64,
) -> anyhow::Result<SignedDelegateAction> {
    use near_primitives::signable_message::{SignableMessage, SignableMessageType};

    let actions = unsigned_transaction
        .actions
        .into_iter()
        .map(near_primitives::action::delegate::NonDelegateAction::try_from)
        .collect::<Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("Delegate action can't contain delegate action"))?;
    let delegate_action = near_primitives::action::delegate::DelegateAction {
        sender_id: unsigned_transaction.signer_id.clone(),
        receiver_id: unsigned_transaction.receiver_id,
        actions,
        nonce: unsigned_transaction.nonce,
        max_block_height,
        public_key: unsigned_transaction.public_key,
    };

    // create a new signature here signing the delegate action + discriminant
    let signable = SignableMessage::new(&delegate_action, SignableMessageType::DelegateAction);
    let signer =
        near_crypto::InMemorySigner::from_secret_key(unsigned_transaction.signer_id, private_key);
    let signature = signable.sign(&signer);

    Ok(near_primitives::action::delegate::SignedDelegateAction {
        delegate_action,
        signature,
    })
}
