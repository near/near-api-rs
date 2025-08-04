/// You can use account key pooling to use different keys for consecutive transactions
/// to avoid nonce-related issues.
///
/// This is an example of how to use account key pooling to send multiple transactions
/// using different keys.
use near_api::{
    Account, NetworkConfig, Signer, Tokens,
    signer::generate_secret_key,
    types::{AccessKeyPermission, AccountId, NearToken},
};
use near_sandbox::{GenesisAccount, SandboxConfig, config::DEFAULT_GENESIS_ACCOUNT};

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let second_account = GenesisAccount::generate_with_name("second_account".parse().unwrap());

    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![second_account.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_sandbox(&sandbox);
    let signer = Signer::from_default_sandbox_account().unwrap();

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
        .unwrap()
        .assert_success();

    signer
        .add_signer_to_pool(Signer::from_secret_key(secret_key))
        .await
        .unwrap();

    let txs = (0..2).map(|_| {
        Tokens::account(account.clone())
            .send_to(second_account.account_id.clone())
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
