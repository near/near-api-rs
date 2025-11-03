/// You can use account key pooling to use different keys for consecutive transactions
/// to avoid nonce-related issues.
///
/// This is an example of how to use account key pooling to send multiple transactions
/// using different keys.
use near_api::{
    signer::generate_secret_key,
    types::{AccessKeyPermission, AccountId, NearToken},
    Account, NetworkConfig, Signer, Tokens,
};
use near_sandbox::{
    config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY},
    GenesisAccount, SandboxConfig,
};

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
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

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
    results.clone().into_iter().for_each(|e| {
        e.assert_success();
    });
    println!(
        "Transaction one public key: {}",
        results[0].transaction().public_key()
    );
    println!(
        "Transaction two public key: {}",
        results[1].transaction().public_key()
    );
    assert_ne!(
        results[0].transaction().public_key(),
        results[1].transaction().public_key()
    );

    println!("All transactions are successful");
}
