use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use near_api::*;

use near_crypto::PublicKey;
use near_primitives::account::AccessKeyPermission;
use signer::generate_secret_key;

#[tokio::test]
async fn multiple_tx_at_same_time_from_same_key() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();

    let tmp_account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let start_nonce = Account(account.id().clone())
        .access_key(Signer::from_workspace(&account).get_public_key().unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;

    let tx = (0..100).map(|i| {
        Tokens::account(account.id().clone())
            .send_to(tmp_account.id().clone())
            .near(NearToken::from_millinear(i))
    });
    let signer = Signer::new(Signer::from_workspace(&account)).unwrap();
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(txs.len(), 100);
    txs.iter().for_each(|a| a.assert_success());

    let end_nonce = Account(account.id().clone())
        .access_key(Signer::from_workspace(&account).get_public_key().unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;
    assert_eq!(end_nonce, start_nonce + 100);
}

#[tokio::test]
async fn multiple_tx_at_same_time_from_different_keys() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let tmp_account = network.dev_create_account().await.unwrap();

    let network = NetworkConfig::from(network);

    let signer = Signer::new(Signer::from_workspace(&account)).unwrap();

    let secret = generate_secret_key().unwrap();
    Account(account.id().clone())
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
    Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess, secret2.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap();
    signer
        .add_signer_to_pool(Signer::from_secret_key(secret2.clone()))
        .await
        .unwrap();

    let tx = (0..12).map(|i| {
        Tokens::account(account.id().clone())
            .send_to(tmp_account.id().clone())
            .near(NearToken::from_millinear(i))
    });
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(txs.len(), 12);
    let mut hash_map = HashMap::new();
    for tx in txs {
        tx.assert_success();
        let public_key = tx.transaction.public_key;
        let count = hash_map.entry(public_key).or_insert(0);
        *count += 1;
    }

    let initial_key = account.secret_key().public_key();
    let initial_key: PublicKey = initial_key.to_string().parse().unwrap();
    assert_eq!(hash_map.len(), 3);
    assert_eq!(hash_map[&initial_key], 4);
    assert_eq!(hash_map[&secret2.public_key()], 4);
    assert_eq!(hash_map[&secret.public_key()], 4);
}
