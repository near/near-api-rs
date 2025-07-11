/// You can use account key pooling to use different keys for consecutive transactions
/// to avoid nonce-related issues.
///
/// This is an example of how to use account key pooling to send multiple transactions
/// using different keys.
use near_api::{
    signer::generate_secret_key,
    types::{AccessKeyPermission, AccountId, NearToken, RpcTransactionResponse},
    *,
};
use near_sandbox_utils::{
    GenesisAccount, SandboxConfig, high_level::config::DEFAULT_GENESIS_ACCOUNT,
};

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let second_account: AccountId = "second_account.near".parse().unwrap();

    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: second_account.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);
    let signer = Signer::new(Signer::default_sandbox()).unwrap();

    println!(
        "Initial public key: {}",
        signer.get_public_key().await.unwrap()
    );

    let secret_key = generate_secret_key().unwrap();
    println!("New public key: {}", secret_key.public_key());

    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, secret_key.public_key())
        .with_signer(Arc::clone(&signer))
        .send_to(&network)
        .await
        .unwrap();

    signer
        .add_signer_to_pool(Signer::from_secret_key(secret_key))
        .await
        .unwrap();

    let txs = (0..2).map(|_| {
        Tokens::account(account.clone())
            .send_to(second_account.clone())
            .near(NearToken::from_millinear(1))
            .with_signer(Arc::clone(&signer))
            .send_to(&network)
    });
    let results = futures::future::join_all(txs)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 2);
    println!("All transactions are successful");
}
