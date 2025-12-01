use near_api::{
    signer::generate_secret_key,
    types::{AccessKeyPermission, AccountId, NearToken, TxExecutionStatus},
    Account, NetworkConfig, Signer, Tokens,
};
use near_sandbox::{
    config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY},
    GenesisAccount, SandboxConfig,
};
use testresult::TestResult;

#[tokio::test]
async fn create_and_delete_account() -> TestResult {
    let network = near_sandbox::Sandbox::start_sandbox().await?;

    let account_id: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let network: NetworkConfig = NetworkConfig::from_rpc_url("sandbox", network.rpc_addr.parse()?);
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    let new_account: AccountId = format!("{}.{}", "bob", account_id).parse()?;
    let secret = generate_secret_key()?;
    let public_key = secret.public_key();

    Account::create_account(new_account.clone())
        .fund_myself(account_id.clone(), NearToken::from_near(1))
        .public_key(public_key)?
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let balance_before_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;

    assert_eq!(balance_before_del.total.as_near(), 1);

    Account(account_id.clone())
        .delete_account_with_beneficiary(new_account.clone())
        .with_signer(signer.clone())
        .wait_until(TxExecutionStatus::Final)
        .send_to(&network)
        .await?
        .assert_success();

    Tokens::account(account_id.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    let balance_after_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;
    assert!(balance_after_del.total > balance_before_del.total);

    Ok(())
}

#[tokio::test]
async fn transfer_funds() -> TestResult {
    let alice: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let bob = GenesisAccount::generate_with_name("bob".parse()?);
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![bob.clone()],
        ..Default::default()
    })
    .await?;
    let network: NetworkConfig = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    Tokens::account(alice.clone())
        .send_to(bob.account_id.clone())
        .near(NearToken::from_near(50))
        .with_signer(Signer::new(Signer::from_secret_key(
            DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
        ))?)
        .send_to(&network)
        .await?
        .assert_success();

    let alice_balance = Tokens::account(alice.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;

    let bob_balance = Tokens::account(bob.account_id.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;

    // it's actually 49.99 because of the fee
    assert_eq!(alice_balance.total.as_near(), 9949);
    assert_eq!(bob_balance.total.as_near(), 10050);

    Ok(())
}

#[tokio::test]
async fn access_key_management() -> TestResult {
    let network = near_sandbox::Sandbox::start_sandbox().await?;
    let network: NetworkConfig = NetworkConfig::from_rpc_url("sandbox", network.rpc_addr.parse()?);
    let alice: AccountId = DEFAULT_GENESIS_ACCOUNT.into();

    let alice_acc = Account(alice.clone());
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    let keys = alice_acc.list_keys().fetch_from(&network).await?;
    assert_eq!(keys.data.len(), 1);

    let secret = generate_secret_key()?;
    let public_key = secret.public_key();

    alice_acc
        .add_key(AccessKeyPermission::FullAccess, public_key.clone())
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await?;
    assert_eq!(keys.data.len(), 2);

    let new_key_info = alice_acc
        .access_key(public_key.clone())
        .fetch_from(&network)
        .await?;

    assert_eq!(
        new_key_info.data.permission,
        AccessKeyPermission::FullAccess
    );

    alice_acc
        .delete_key(secret.public_key())
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await?;

    assert_eq!(keys.data.len(), 1);

    alice_acc
        .access_key(secret.public_key())
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    for _ in 0..10 {
        let secret = generate_secret_key()?;
        alice_acc
            .add_key(AccessKeyPermission::FullAccess, secret.public_key())
            .with_signer(signer.clone())
            .send_to(&network)
            .await?
            .assert_success();
    }

    let keys = alice_acc.list_keys().fetch_from(&network).await?;

    assert_eq!(keys.data.len(), 11);

    alice_acc
        .delete_keys(
            keys.data
                .into_iter()
                .map(|(public_key, _)| public_key)
                .collect(),
        )
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await?;
    assert_eq!(keys.data.len(), 0);

    Ok(())
}
