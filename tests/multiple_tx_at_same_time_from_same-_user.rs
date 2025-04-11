use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use near_api::*;

use near_primitives::account::AccessKeyPermission;
use signer::generate_secret_key;

#[tokio::test]
async fn multiple_tx_at_same_time_from_same_key() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account).await.unwrap();
    let network = &sandbox.network_config;

    let tmp_account = "tmp_account.sandbox".parse().unwrap();
    let _tmp_account_sk = sandbox.create_root_subaccount(&tmp_account).await.unwrap();

    let start_nonce = Account(account.clone())
        .access_key(account_sk.public_key())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;

    let tx = (0..100).map(|i| {
        Tokens::account(account.clone())
            .send_to(tmp_account.clone())
            .near(NearToken::from_millinear(i))
    });
    let signer = Signer::new(Signer::from_secret_key(account_sk.clone())).unwrap();
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(txs.len(), 100);
    txs.iter().for_each(|a| a.assert_success());

    let end_nonce = Account(account.clone())
        .access_key(account_sk.public_key())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;
    assert_eq!(end_nonce, start_nonce + 100);
}

#[tokio::test]
async fn multiple_tx_at_same_time_from_different_keys() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account).await.unwrap();
    let network = &sandbox.network_config;

    let tmp_account = "tmp_account.sandbox".parse().unwrap();
    let _tmp_account_sk = sandbox.create_root_subaccount(&tmp_account).await.unwrap();

    let signer = Signer::new(Signer::from_secret_key(account_sk.clone())).unwrap();

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
        .unwrap();
    signer
        .add_signer_to_pool(Signer::from_secret_key(secret2.clone()))
        .await
        .unwrap();

    let tx = (0..12).map(|i| {
        Tokens::account(account.clone())
            .send_to(tmp_account.clone())
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

    assert_eq!(hash_map.len(), 3);
    assert_eq!(hash_map[&account_sk.public_key()], 4);
    assert_eq!(hash_map[&secret2.public_key()], 4);
    assert_eq!(hash_map[&secret.public_key()], 4);
}
