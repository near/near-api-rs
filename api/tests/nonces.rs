use std::sync::Arc;

use futures::future::join_all;
use near_api::*;
use near_api_types::{AccountId, NearToken};
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use testresult::TestResult;

#[tokio::test]
async fn correct_nonces_for_different_networks() -> TestResult {
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?, None)?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let second_sandbox = near_sandbox::Sandbox::start_sandbox().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let second_network =
        NetworkConfig::from_rpc_url("second_sandbox", second_sandbox.rpc_addr.parse()?);

    let tx = Tokens::account(account.clone())
        .send_to("tmp_account".parse()?)
        .near(NearToken::from_millinear(1));

    tx.clone()
        .with_signer(signer.clone())
        .presign_with(&network)
        .await?;

    let nonce_before = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    tx.with_signer(signer.clone())
        .presign_with(&second_network)
        .await?;

    let nonce_after = Account(account.clone())
        .access_key(signer.get_public_key().await?)
        .fetch_from(&network)
        .await?
        .data
        .nonce;

    // Check that nonce differs for the same account but different networks
    assert_eq!(nonce_after.0, nonce_before.0);

    Ok(())
}

#[tokio::test]
async fn sequential_nonces() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?, None)?;

    let tx_count = 10;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let tx = Tokens::account(account.clone())
        .send_to(receiver.clone())
        .near(NearToken::from_millinear(1));

    // Commit sequential nonce to signer
    join_all((0..tx_count).map(|_| {
        tx.clone()
            .with_signer(Arc::clone(&signer))
            .send_to(&network)
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    // Try to presign with non sequential nonce
    tx.with_signer(Arc::clone(&signer))
        .presign_offline(signer.get_public_key().await?, CryptoHash::default(), 0)
        .await?
        .send_to(&network)
        .await
        .err()
        .ok_or("Should not be able to use with non sequential nonce")?;

    Ok(())
}
