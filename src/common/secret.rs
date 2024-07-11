use std::path::PathBuf;

use near_crypto::{PublicKey, SecretKey};
use slipped10::BIP32Path;

use crate::sign::{get_secret_key_from_seed, Signer};

const DEFAULT_HD_PATH: &str = "m/44'/397'/0'";
const DEFAULT_WORD_COUNT: usize = 12;

pub type SecretCallback<T> = dyn FnOnce(PublicKey) -> anyhow::Result<T>;

pub struct SecretBuilder<T> {
    next_step: Box<SecretCallback<T>>,
}

impl<T> SecretBuilder<T> {
    pub fn new(next_step: Box<SecretCallback<T>>) -> Self {
        Self { next_step }
    }

    pub fn auto_generate(self) -> AutoGenerateBuilder<T> {
        AutoGenerateBuilder {
            next_step: self.next_step,
            master_seed_phrase: None,
            word_count: None,
            hd_path: None,
            passphrase: None,
        }
    }
}

pub struct AutoGenerateBuilder<T> {
    next_step: Box<SecretCallback<T>>,

    pub master_seed_phrase: Option<String>,
    pub word_count: Option<usize>,
    pub hd_path: Option<BIP32Path>,
    pub passphrase: Option<String>,
}

impl<T> AutoGenerateBuilder<T> {
    pub fn master_seed_phrase(mut self, master_seed_phrase: String) -> Self {
        self.master_seed_phrase = Some(master_seed_phrase);
        self
    }

    pub fn word_count(mut self, word_count: usize) -> Self {
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

    pub fn with_seed_phrase(self) -> anyhow::Result<(String, T)> {
        let master_seed_phrase =
            if let Some(master_seed_phrase) = self.master_seed_phrase.as_deref() {
                master_seed_phrase.to_owned()
            } else {
                let mnemonic =
                    bip39::Mnemonic::generate(self.word_count.unwrap_or(DEFAULT_WORD_COUNT))?;
                mnemonic.word_iter().collect::<Vec<&str>>().join(" ")
            };

        let signer = Signer::seed_phrase_with_hd_path(
            master_seed_phrase.clone(),
            self.hd_path
                .unwrap_or(DEFAULT_HD_PATH.parse().expect("Valid HD path")),
            self.passphrase,
        )?;

        let pk = signer.as_signer().get_public_key()?;

        Ok((master_seed_phrase, (self.next_step)(pk)?))
    }

    pub fn with_secret_key(self) -> anyhow::Result<(SecretKey, T)> {
        let hd_path = self
            .hd_path
            .clone()
            .unwrap_or(DEFAULT_HD_PATH.parse().expect("Valid HD path"));
        let passphrase = self.passphrase.clone();
        let (seed_phrase, next) = self.with_seed_phrase()?;

        let secret_key = get_secret_key_from_seed(hd_path, seed_phrase, passphrase)?;

        Ok((secret_key, next))
    }

    pub fn save_to_file(self, path: PathBuf) -> anyhow::Result<T> {
        let (seed, next) = self.with_seed_phrase()?;
        std::fs::write(path, seed)?;
        Ok(next)
    }
}
