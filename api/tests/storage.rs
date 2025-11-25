use near_api::*;

use near_api_types::{AccountId, Data, NearToken, StorageBalance};
use near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;
use std::sync::Arc;

struct TestContext {
    _sandbox: near_sandbox::Sandbox,
    network: NetworkConfig,
    account: AccountId,
    storage: StorageDeposit,
    signer: Arc<Signer>,
}

async fn setup_social_contract() -> TestContext {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    let contract = Contract("social.near".parse().unwrap());
    let account: AccountId = "user.sandbox".parse().unwrap();

    sandbox
        .create_account(account.clone())
        .send()
        .await
        .unwrap();

    sandbox
        .import_account(RPCEndpoint::mainnet().url, contract.account_id().clone())
        .send()
        .await
        .unwrap();

    contract
        .call_function("new", ())
        .unwrap()
        .transaction()
        .with_signer(contract.account_id().clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    contract
        .call_function("set_status", serde_json::json!({ "status": "Live" }))
        .unwrap()
        .transaction()
        .with_signer(contract.account_id().clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    TestContext {
        storage: StorageDeposit::on_contract(contract.account_id().clone()),
        _sandbox: sandbox,
        network,
        account,
        signer,
    }
}

#[tokio::test]
async fn test_that_generic_account_has_no_storage() {
    let ctx: TestContext = setup_social_contract().await;

    let balance: Data<Option<StorageBalance>> = ctx
        .storage
        .view_account_storage(ctx.account.clone())
        .unwrap()
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    assert!(balance.data.is_none());
}

#[tokio::test]
async fn test_deposit_integration() {
    let ctx = setup_social_contract().await;

    // Make a storage deposit for ourselves
    let deposit_amount = NearToken::from_near(10);
    ctx.storage
        .deposit(ctx.account.clone(), deposit_amount)
        .unwrap()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    // Verify the storage balance increased
    let balance: Data<Option<StorageBalance>> = ctx
        .storage
        .view_account_storage(ctx.account.clone())
        .unwrap()
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    assert!(balance.data.is_some());
    assert_eq!(balance.data.unwrap().total, deposit_amount);
}

#[tokio::test]
async fn test_withdraw_integration() {
    let ctx = setup_social_contract().await;

    // First deposit storage
    let deposit_amount = NearToken::from_near(10);
    ctx.storage
        .deposit(ctx.account.clone(), deposit_amount)
        .unwrap()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    // Get initial balance
    let initial_balance: Data<Option<StorageBalance>> = ctx
        .storage
        .view_account_storage(ctx.account.clone())
        .unwrap()
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    let initial_available = initial_balance.data.as_ref().unwrap().total;

    // Try to withdraw (might fail if there's no available balance, but tests the flow)
    ctx.storage
        .withdraw(ctx.account.clone(), NearToken::from_yoctonear(1000))
        .unwrap()
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    let balance: Data<Option<StorageBalance>> = ctx
        .storage
        .view_account_storage(ctx.account.clone())
        .unwrap()
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    assert_eq!(
        balance.data.unwrap().total,
        initial_available.saturating_sub(NearToken::from_yoctonear(1000))
    );
}
