use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use near_api::*;
use near_api_types::{AccessKeyPermission, AccountId, NearToken};
use near_sandbox::{
    GenesisAccount, SandboxConfig,
    config::{
        DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY,
        DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY,
    },
};
use signer::generate_secret_key;

#[tokio::test]
async fn multiple_tx_at_same_time_from_same_key() {
    let tmp_account = GenesisAccount::generate_with_name("tmp_account".parse().unwrap());
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![tmp_account.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    let start_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await.unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;

    let tx = (0..100).map(|i| {
        Tokens::account(account.clone())
            .send_to(tmp_account.account_id.clone())
            .near(NearToken::from_millinear(i))
    });
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(txs.len(), 100);

    let end_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await.unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;
    assert_eq!(end_nonce.0, start_nonce.0 + 100);
}

#[tokio::test]
async fn multiple_tx_at_same_time_from_different_keys() {
    let tmp_account = GenesisAccount::generate_with_name("tmp_account".parse().unwrap());
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![tmp_account.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    let secret = generate_secret_key().unwrap();
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, secret.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    signer
        .add_signer_to_pool(Signer::from_secret_key(secret.clone()))
        .await
        .unwrap();

    let secret2 = generate_secret_key().unwrap();
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, secret2.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();
    signer
        .add_signer_to_pool(Signer::from_secret_key(secret2.clone()))
        .await
        .unwrap();

    let tx = (0..12).map(|i| {
        Tokens::account(account.clone())
            .send_to(tmp_account.account_id.clone())
            .near(NearToken::from_millinear(i))
    });
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .map(|t| t.unwrap().assert_success())
        .collect::<Vec<_>>();

    assert_eq!(txs.len(), 12);
    let mut hash_map = HashMap::new();
    for tx in txs {
        let public_key = tx.transaction().public_key();
        let count: &mut i32 = hash_map.entry(public_key.to_string()).or_insert(0);
        *count += 1;
    }

    assert_eq!(hash_map.len(), 3);
    assert_eq!(hash_map[DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY], 4);
    assert_eq!(hash_map[&secret2.public_key().to_string()], 4);
    assert_eq!(hash_map[&secret.public_key().to_string()], 4);
}
