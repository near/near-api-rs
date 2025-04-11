use near_api::*;
use near_primitives::account::AccessKeyPermission;
use signer::generate_seed_phrase;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let network = &sandbox.network_config;

    // Current secret key from workspace
    let (new_seed_phrase, public_key) = generate_seed_phrase().unwrap();

    // Let's add new key and get the seed phrase
    Account(account_id.clone())
        .add_key(AccessKeyPermission::FullAccess, public_key)
        .with_signer(Signer::new(Signer::from_secret_key(account_sk.clone())).unwrap())
        .send_to(network)
        .await
        .unwrap();

    // Let's add ledger to the account with the new seed phrase
    let ledger_pubkey = Signer::from_ledger().get_public_key().unwrap();
    Account(account_id.clone())
        .add_key(AccessKeyPermission::FullAccess, ledger_pubkey)
        .with_signer(
            Signer::new(Signer::from_seed_phrase(&new_seed_phrase, Some("smile")).unwrap())
                .unwrap(),
        )
        .send_to(network)
        .await
        .unwrap();

    println!("Signing with ledger");
    // Let's sign some tx with the ledger key
    Account(account_id.clone())
        .delete_key(account_sk.public_key())
        .with_signer(Signer::new(Signer::from_ledger()).unwrap())
        .send_to(network)
        .await
        .unwrap();

    let keys = Account(account_id.clone())
        .list_keys()
        .fetch_from(network)
        .await
        .unwrap();

    // Should contain 2 keys: new key from seed phrase, and ledger key
    println!("{:#?}", keys);
}
