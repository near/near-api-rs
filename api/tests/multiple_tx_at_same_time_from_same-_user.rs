use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use near_api::*;
use near_api_types::{AccessKeyPermission, AccountId, NearToken};
use near_sandbox::config::{
    DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY,
    DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY,
};
use signer::generate_secret_key;
use testresult::TestResult;

#[tokio::test]
async fn multiple_tx_at_same_time_from_same_key() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;

    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    let start_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    let tx = (0..20).map(|i| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(i))
    });
    // Even though we send 20 transactions with correct nonces, it still might fail
    // because of the blockchain/network inclusion race condition
    //
    // TX1 gets nonce=100, TX2 gets nonce=101, TX3 gets nonce=102, TX4 gets nonce=103
    // TX4 (nonce=103) arrives and gets included in the block at validator first ✓
    // TX1 (nonce=100) arrives second ✗ (rejected - nonce too old, expected 104)
    // TX3 (nonce=102) arrives third ✗ (rejected - nonce too old, expected 104)
    // TX2 (nonce=101) arrives last ✗ (rejected - nonce too old, expected 104)
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(txs.len(), 20);

    let end_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;
    assert_eq!(end_nonce.0, start_nonce.0 + 20);

    Ok(())
}

#[tokio::test]
async fn multiple_tx_at_same_time_from_different_keys() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    let secret = generate_secret_key()?;
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, secret.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    signer
        .add_signer_to_pool(Signer::from_secret_key(secret.clone()))
        .await?;

    let secret2 = generate_secret_key()?;
    Account(account.clone())
        .add_key(AccessKeyPermission::FullAccess, secret2.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();
    signer
        .add_signer_to_pool(Signer::from_secret_key(secret2.clone()))
        .await?;

    let tx = (0..12).map(|i| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(i))
    });
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .map(|t| t.map(|t| t.assert_success()))
        .collect::<Result<Vec<_>, _>>()?;

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

    Ok(())
}
