/// You can use account key pooling to use different keys for consecutive transactions
/// to avoid nonce-related issues.
///
/// This is an example of how to use account key pooling to send multiple transactions
/// using different keys.
use near_api::*;
use near_token::NearToken;
use signer::generate_secret_key;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let second_account_id = "second_account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let _second_account = sandbox
        .create_root_subaccount(&second_account_id)
        .await
        .unwrap();
    let network = &sandbox.network_config;

    let signer = Signer::new(Signer::from_secret_key(account_sk)).unwrap();

    println!(
        "Initial public key: {}",
        signer.get_public_key().await.unwrap()
    );

    let secret_key = generate_secret_key().unwrap();
    println!("New public key: {}", secret_key.public_key());

    Account(account_id.clone())
        .add_key(
            near_primitives::account::AccessKeyPermission::FullAccess,
            secret_key.public_key(),
        )
        .with_signer(Arc::clone(&signer))
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    signer
        .add_signer_to_pool(Signer::from_secret_key(secret_key))
        .await
        .unwrap();

    let txs = (0..2).map(|_| {
        Tokens::account(account_id.clone())
            .send_to(second_account_id.clone())
            .near(NearToken::from_near(1))
            .with_signer(Arc::clone(&signer))
            .send_to(network)
    });
    let results = futures::future::join_all(txs)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 2);
    results.iter().for_each(|e| e.assert_success());
    println!("All transactions are successful");
    println!(
        "Transaction one public key: {}",
        results[0].transaction.public_key
    );
    println!(
        "Transaction two public key: {}",
        results[1].transaction.public_key
    );
    assert_ne!(
        results[0].transaction.public_key,
        results[1].transaction.public_key
    );

    println!("All transactions are successful");
}
