use std::path::PathBuf;

use near_crypto::{PublicKey, SecretKey};
use slipped10::BIP32Path;

use crate::{
    errors::{SecretBuilderkError, SecretError},
    signer::{get_secret_key_from_seed, Signer, SignerTrait},
};

const DEFAULT_HD_PATH: &str = "m/44'/397'/0'";
const DEFAULT_WORD_COUNT: usize = 12;

pub type SecretCallback<T, E> = dyn FnOnce(PublicKey) -> Result<T, E>;

pub struct SecretBuilder<T, E> {
    next_step: Box<SecretCallback<T, E>>,
}

impl<T, E> SecretBuilder<T, E>
where
    E: std::fmt::Debug + std::fmt::Display,
{
    pub fn new<Fn>(next_step: Fn) -> Self
    where
        Fn: FnOnce(PublicKey) -> Result<T, E> + 'static,
    {
        Self {
            next_step: Box::new(next_step),
        }
    }

    pub fn new_keypair(self) -> GenerateKeypairBuilder<T, E> {
        GenerateKeypairBuilder {
            next_step: self.next_step,
            master_seed_phrase: None,
            word_count: None,
            hd_path: None,
            passphrase: None,
        }
    }

    pub fn use_public_key_from(
        self,
        signer: &dyn SignerTrait,
    ) -> Result<T, SecretBuilderkError<E>> {
        let pk: PublicKey = signer
            .get_public_key()
            .map_err(|_| SecretBuilderkError::PublicKeyIsNotAvailable)?;
        (self.next_step)(pk).map_err(|e| SecretBuilderkError::CallbackError(e))
    }

    pub fn use_public_key(self, pk: PublicKey) -> Result<T, SecretBuilderkError<E>> {
        (self.next_step)(pk).map_err(|e| SecretBuilderkError::CallbackError(e))
    }
}

pub struct GenerateKeypairBuilder<T, E> {
    next_step: Box<SecretCallback<T, E>>,

    pub master_seed_phrase: Option<String>,
    pub word_count: Option<usize>,
    pub hd_path: Option<BIP32Path>,
    pub passphrase: Option<String>,
}

impl<T, E> GenerateKeypairBuilder<T, E>
where
    E: std::fmt::Debug + std::fmt::Display,
{
    pub fn master_seed_phrase(mut self, master_seed_phrase: String) -> Self {
        self.master_seed_phrase = Some(master_seed_phrase);
        self
    }

    pub const fn word_count(mut self, word_count: usize) -> Self {
        self.word_count = Some(word_count);
        self
    }

    pub fn hd_path(mut self, hd_path: BIP32Path) -> Self {
        self.hd_path = Some(hd_path);
        self
    }

    pub fn passphrase(mut self, passphrase: String) -> Self {
        self.passphrase = Some(passphrase);
        self
    }

    pub fn generate_seed_phrase(self) -> Result<(String, T), SecretBuilderkError<E>> {
        let master_seed_phrase = if let Some(master_seed_phrase) =
            self.master_seed_phrase.as_deref()
        {
            master_seed_phrase.to_owned()
        } else {
            let mnemonic = bip39::Mnemonic::generate(self.word_count.unwrap_or(DEFAULT_WORD_COUNT))
                .map_err(SecretError::from)?;
            mnemonic.word_iter().collect::<Vec<&str>>().join(" ")
        };

        let signer = Signer::seed_phrase_with_hd_path(
            master_seed_phrase.clone(),
            self.hd_path
                .unwrap_or_else(|| DEFAULT_HD_PATH.parse().expect("Valid HD path")),
            self.passphrase,
        )?;

        let pk = signer
            .get_public_key()
            .map_err(|_| SecretBuilderkError::PublicKeyIsNotAvailable)?;

        Ok((
            master_seed_phrase,
            (self.next_step)(pk).map_err(|e| SecretBuilderkError::CallbackError(e))?,
        ))
    }

    pub fn generate_secret_key(self) -> Result<(SecretKey, T), SecretBuilderkError<E>> {
        let hd_path = self
            .hd_path
            .clone()
            .unwrap_or_else(|| DEFAULT_HD_PATH.parse().expect("Valid HD path"));
        let passphrase = self.passphrase.clone();
        let (seed_phrase, next) = self.generate_seed_phrase()?;

        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, passphrase)?;

        Ok((secret_key, next))
    }

    pub fn save_generated_seed_to_file(self, path: PathBuf) -> Result<T, SecretBuilderkError<E>> {
        let (seed, next) = self.generate_seed_phrase()?;
        std::fs::write(path, seed)?;
        Ok(next)
    }
}
