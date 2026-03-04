use std::sync::Arc;

use futures::future::join_all;
use near_api::*;
use near_api_types::{AccessKeyPermission, AccountId, NearToken};
use near_sandbox::config::{
    DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY,
    DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY,
};
use std::str::FromStr;
use testresult::TestResult;

#[tokio::test]
async fn multiple_sequential_tx_at_same_time_from_same_key() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let start_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    signer.set_sequential(true);

    let tx_count = 100;
    let tx = (0..tx_count).map(|_| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(1))
    });

    join_all(tx.map(|t| {
        t.with_signer(Arc::clone(&signer))
            .wait_until(near_api_types::TxExecutionStatus::Final)
            .send_to(&network)
    }))
    .await
    .into_iter()
    .map(|t| t.map(|t| t.assert_success()))
    .collect::<Result<Vec<_>, _>>()?;

    let end_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;
    assert_eq!(end_nonce.0, start_nonce.0 + tx_count as u64);

    Ok(())
}

#[tokio::test]
async fn multiple_non_sequential_tx_at_same_time_from_same_key() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let tx_count = 20;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let start_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    let tx = (0..tx_count).map(|_| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(1))
    });
    // Even though we send multiple transactions with correct nonces, it still might fail
    // because of the blockchain/network inclusion race condition
    //
    // TX1 gets nonce=100, TX2 gets nonce=101, TX3 gets nonce=102, TX4 gets nonce=103
    // TX4 (nonce=103) arrives and gets included in the block at validator first ✓
    // TX1 (nonce=100) arrives second ✗ (rejected - nonce too old, expected 104)
    // TX3 (nonce=102) arrives third ✗ (rejected - nonce too old, expected 104)
    // TX2 (nonce=101) arrives last ✗ (rejected - nonce too old, expected 104)
    join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network))).await;

    let end_nonce = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;
    assert_eq!(end_nonce.0, start_nonce.0 + tx_count as u64);

    Ok(())
}

#[tokio::test]
async fn multiple_sequential_tx_at_same_time_from_different_keys() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let pubkey_count = 9;
    let tx_count = 1000;
    let first_pubkey = PublicKey::from_str(DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY)?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    signer.set_sequential(true);

    join_all((0..pubkey_count).map(|_| add_key_to_pool(&account, &signer, &network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let start_nonce = Account(account.clone())
        .access_key(first_pubkey)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    let tx = (0..tx_count).map(|_| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(1))
    });

    join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .map(|t| t.map(|t| t.assert_success()))
        .collect::<Result<Vec<_>, _>>()?;

    let end_nonce = Account(account.clone())
        .access_key(first_pubkey)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    assert_eq!(
        end_nonce.0,
        start_nonce.0 + tx_count as u64 / (pubkey_count + 1)
    );

    Ok(())
}

#[tokio::test]
async fn multiple_non_sequential_tx_at_same_time_from_different_keys() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let pubkey_count = 9;
    let tx_count = 20;
    let first_pubkey = PublicKey::from_str(DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY)?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    join_all((0..pubkey_count).map(|_| add_key_to_pool(&account, &signer, &network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let start_nonce = Account(account.clone())
        .access_key(first_pubkey)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    let tx = (0..tx_count).map(|_| {
        Tokens::account(account.clone())
            .send_to(receiver.clone())
            .near(NearToken::from_millinear(1))
    });

    // Even though we send multiple transactions with correct nonces, it still might fail
    // because of the blockchain/network inclusion race condition
    //
    // TX1 gets nonce=100, TX2 gets nonce=101, TX3 gets nonce=102, TX4 gets nonce=103
    // TX4 (nonce=103) arrives and gets included in the block at validator first ✓
    // TX1 (nonce=100) arrives second ✗ (rejected - nonce too old, expected 104)
    // TX3 (nonce=102) arrives third ✗ (rejected - nonce too old, expected 104)
    // TX2 (nonce=101) arrives last ✗ (rejected - nonce too old, expected 104)

    join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network))).await;

    let end_nonce = Account(account.clone())
        .access_key(first_pubkey)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    let expected_per_key = tx_count as u64 / (pubkey_count + 1);

    assert_eq!(end_nonce.0, start_nonce.0 + expected_per_key);

    Ok(())
}

async fn add_key_to_pool(
    account_id: &AccountId,
    signer: &Arc<Signer>,
    network: &NetworkConfig,
) -> TestResult {
    let secret = signer::generate_secret_key()?;
    Account(account_id.clone())
        .add_key(AccessKeyPermission::FullAccess, secret.public_key())
        .with_signer(Arc::clone(signer))
        .send_to(network)
        .await?
        .assert_success();

    signer.add_secret_key_to_pool(secret).await?;

    Ok(())
}
