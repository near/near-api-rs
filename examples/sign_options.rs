use near_api::{prelude::*, types::views::AccessKeyPermission};
use near_crypto::SecretKey;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    // Current secret key from workspace
    let current_secret_key: SecretKey = account.secret_key().to_string().parse().unwrap();

    // Let's add new key and get the seed phrase
    let (new_seed_phrase, tx) = Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess)
        .new_keypair()
        // Passphrase is optional. You can also configure hd_path
        .passphrase("smile".to_string())
        .generate_seed_phrase()
        .unwrap();
    tx.with_signer(Signer::new(Signer::secret_key(current_secret_key.clone())).unwrap())
        .send_to(&network)
        .await
        .unwrap();

    // Let's add ledger to the account with the new seed phrase
    Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess)
        .use_public_key_from(&Signer::ledger())
        .unwrap()
        .with_signer(
            Signer::new(Signer::seed_phrase(new_seed_phrase, Some("smile".to_string())).unwrap())
                .unwrap(),
        )
        .send_to(&network)
        .await
        .unwrap();

    println!("Signing with ledger");
    // Let's sign some tx with the ledger key
    Account(account.id().clone())
        .delete_key(current_secret_key.public_key())
        .with_signer(Signer::new(Signer::ledger()).unwrap())
        .send_to(&network)
        .await
        .unwrap();

    let keys = Account(account.id().clone())
        .list_keys()
        .fetch_from(&network)
        .await
        .unwrap();

    // Should contain 2 keys: new key from seed phrase, and ledger key
    println!("{:#?}", keys);
}
