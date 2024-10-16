use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use near_api::{
    signer::{Signer, SignerTrait},
    Account, NetworkConfig, Tokens,
};
use near_crypto::PublicKey;
use near_primitives::account::AccessKeyPermission;
use near_token::NearToken;

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
        Tokens::of(account.id().clone())
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

    let (key1, acc) = Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess)
        .new_keypair()
        .generate_secret_key()
        .unwrap();
    acc.with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    signer
        .add_signer_to_pool(Signer::secret_key(key1.clone()))
        .await
        .unwrap();

    let (key2, acc) = Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess)
        .new_keypair()
        .generate_secret_key()
        .unwrap();
    let result = acc
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap();
    signer
        .add_signer_to_pool(Signer::secret_key(key2.clone()))
        .await
        .unwrap();

    result.assert_success();

    let tx = (0..12).map(|i| {
        Tokens::of(account.id().clone())
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
    assert_eq!(hash_map[&key1.public_key()], 4);
    assert_eq!(hash_map[&key2.public_key()], 4);
}
