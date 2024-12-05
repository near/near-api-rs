use near_crypto::{PublicKey, SecretKey};
use slipped10::BIP32Path;

use crate::{errors::SecretError, signer::get_secret_key_from_seed};

const DEFAULT_HD_PATH: &str = "m/44'/397'/0'";
const DEFAULT_WORD_COUNT: usize = 12;

/// Generates a new seed phrase with optional customization
pub fn generate_seed_phrase_custom(
    word_count: Option<usize>,
    hd_path: Option<BIP32Path>,
    passphrase: Option<String>,
) -> Result<(String, PublicKey), SecretError> {
    let mnemonic = bip39::Mnemonic::generate(word_count.unwrap_or(DEFAULT_WORD_COUNT))?;
    let seed_phrase = mnemonic.words().collect::<Vec<&str>>().join(" ");

    let secret_key = get_secret_key_from_seed(
        hd_path.unwrap_or_else(|| DEFAULT_HD_PATH.parse().expect("Valid HD path")),
        seed_phrase.clone(),
        passphrase,
    )?;

    Ok((seed_phrase, secret_key.public_key()))
}

/// Generates a new seed phrase with default settings (12 words, default HD path)
pub fn generate_seed_phrase() -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, None)
}

/// Generates a new seed phrase with a custom HD path
pub fn generate_seed_phrase_with_hd_path(
    hd_path: BIP32Path,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, Some(hd_path), None)
}

/// Generates a new seed phrase with a custom passphrase
pub fn generate_seed_phrase_with_passphrase(
    passphrase: String,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(None, None, Some(passphrase))
}

/// Generates a new seed phrase with a custom word count
pub fn generate_seed_phrase_with_word_count(
    word_count: usize,
) -> Result<(String, PublicKey), SecretError> {
    generate_seed_phrase_custom(Some(word_count), None, None)
}

/// Generates a secret key from a new seed phrase using default settings
pub fn generate_secret_key() -> Result<SecretKey, SecretError> {
    let (seed_phrase, _) = generate_seed_phrase()?;
    let secret_key = get_secret_key_from_seed(
        DEFAULT_HD_PATH.parse().expect("Valid HD path"),
        seed_phrase,
        None,
    )?;
    Ok(secret_key)
}

pub fn generate_secret_key_from_seed_phrase(seed_phrase: String) -> Result<SecretKey, SecretError> {
    get_secret_key_from_seed(
        DEFAULT_HD_PATH.parse().expect("Valid HD path"),
        seed_phrase,
        None,
    )
}
