/// You can use account key pooling to use different keys for consecutive transactions
/// to avoid nonce-related issues.
///
/// This is an example of how to use account key pooling to send multiple transactions
/// using different keys.
use near_api::prelude::*;
use near_token::NearToken;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let second_account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let signer = Signer::new(Signer::from_workspace(&account)).unwrap();

    println!(
        "Initial public key: {}",
        signer.get_public_key().await.unwrap()
    );

    let secret_key = generate_secret_key().unwrap();
    println!("New public key: {}", secret_key.public_key());

    Account(account.id().clone())
        .add_key(
            near_primitives::account::AccessKeyPermission::FullAccess,
            secret_key.public_key(),
        )
        .with_signer(Arc::clone(&signer))
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    signer
        .add_signer_to_pool(Signer::secret_key(secret_key))
        .await
        .unwrap();

    let txs = (0..2).map(|_| {
        Tokens::of(account.id().clone())
            .send_to(second_account.id().clone())
            .near(NearToken::from_near(1))
            .with_signer(Arc::clone(&signer))
            .send_to(&network)
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
