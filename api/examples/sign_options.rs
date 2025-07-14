use near_api::{
    signer::generate_secret_key,
    types::{AccessKeyPermission, AccountId, SecretKey},
    *,
};
use near_sandbox_utils::high_level::config::DEFAULT_GENESIS_ACCOUNT;
use signer::generate_seed_phrase;

#[tokio::main]
async fn main() {
    let network = near_sandbox_utils::high_level::Sandbox::start_sandbox()
        .await
        .unwrap();
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    // Current secret key from workspace
    let current_secret_key: SecretKey = generate_secret_key().unwrap();
    let (new_seed_phrase, public_key) = generate_seed_phrase().unwrap();

    // Let's add new key and get the seed phrase
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, public_key)
        .with_signer(Signer::new(Signer::from_secret_key(current_secret_key.clone())).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Let's add ledger to the account with the new seed phrase
    let ledger_pubkey = Signer::from_ledger().get_public_key().unwrap();
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
        .delete_key(current_secret_key.public_key())
        .with_signer(Signer::new(Signer::from_ledger()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let keys = Account(account.clone())
        .list_keys()
        .fetch_from(&network)
        .await
        .unwrap();

    // Should contain 2 keys: new key from seed phrase, and ledger key
    println!("{keys:#?}");
}
