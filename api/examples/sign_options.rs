use std::str::FromStr;

use near_api::{
    Account, NetworkConfig, PublicKey, Signer, SignerTrait,
    signer::generate_seed_phrase_with_passphrase,
    types::{AccessKeyPermission, AccountId},
};
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY};

#[tokio::main]
async fn main() {
    let network = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let network = NetworkConfig::from_sandbox(&network);

    // Current secret key from workspace
    let (new_seed_phrase, public_key) = generate_seed_phrase_with_passphrase("smile").unwrap();

    // Let's add new key and get the seed phrase
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, public_key)
        .with_signer(Signer::from_default_sandbox_account().unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    if std::env::var("CI").is_ok() {
        println!("Skipping ledger signing in CI");
    } else {
        // Let's add ledger to the account with the new seed phrase
        let ledger = Signer::from_ledger();
        let ledger_pubkey = ledger.get_public_key().unwrap();
        Account(account.clone())
            .add_key(AccessKeyPermission::FullAccess, ledger_pubkey)
            .with_signer(
                Signer::new(Signer::from_seed_phrase(&new_seed_phrase, Some("smile")).unwrap())
                    .unwrap(),
            )
            .send_to(&network)
            .await
            .unwrap()
            .assert_success();

        println!("Signing with ledger");

        // Let's sign some tx with the ledger key
        Account(account.clone())
            .delete_key(PublicKey::from_str(DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY).unwrap())
            .with_signer(Signer::new(ledger).unwrap())
            .send_to(&network)
            .await
            .unwrap()
            .assert_success();
    }

    let keys = Account(account.clone())
        .list_keys()
        .fetch_from(&network)
        .await
        .unwrap();

    // Should contain 2 keys: new key from seed phrase, and ledger key
    println!("{keys:#?}");
    assert_eq!(keys.data.len(), 2);
}
