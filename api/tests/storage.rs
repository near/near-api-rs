use near_api_types::{Data, NearToken, StorageBalance};

mod common;

use common::{setup_ft_contract, setup_social_contract, TestContext};

#[tokio::test]
async fn test_that_generic_account_has_no_storage() {
    let ctx: TestContext = setup_social_contract().await;

    let balance: Data<Option<StorageBalance>> = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
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
    ctx.contract
        .storage_deposit()
        .deposit(ctx.account.clone(), deposit_amount)
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    // Verify the storage balance increased
    let balance: Data<Option<StorageBalance>> = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
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
    ctx.contract
        .storage_deposit()
        .deposit(ctx.account.clone(), deposit_amount)
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    // Get initial balance
    let initial_balance: Data<Option<StorageBalance>> = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    let initial_available = initial_balance.data.as_ref().unwrap().total;

    // Try to withdraw (might fail if there's no available balance, but tests the flow)
    ctx.contract
        .storage_deposit()
        .withdraw(ctx.account.clone(), NearToken::from_yoctonear(1000))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    let balance: Data<Option<StorageBalance>> = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    assert_eq!(
        balance.data.unwrap().total,
        initial_available.saturating_sub(NearToken::from_yoctonear(1000))
    );
}

#[tokio::test]
async fn test_unregister_integration() {
    // Social doesn't support unregistering
    let ctx = setup_ft_contract().await;

    // First deposit storage
    let deposit_amount = NearToken::from_near(10);
    ctx.contract
        .storage_deposit()
        .deposit(ctx.account.clone(), deposit_amount)
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    let balance = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
        .fetch_from(&ctx.network)
        .await
        .unwrap();
    assert!(balance.data.is_some());

    ctx.contract
        .storage_deposit()
        .unregister()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    let balance = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
        .fetch_from(&ctx.network)
        .await
        .unwrap();
    assert!(balance.data.is_none());
}

#[tokio::test]
async fn test_registration_only_integration() {
    let ctx = setup_social_contract().await;

    let deposit_amount = NearToken::from_near(10);
    ctx.contract
        .storage_deposit()
        .deposit(ctx.account.clone(), deposit_amount)
        .registration_only()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    let balance = ctx
        .contract
        .storage_deposit()
        .view_account_storage(ctx.account.clone())
        .fetch_from(&ctx.network)
        .await
        .unwrap();

    assert!(
        balance.data.as_ref().unwrap().total < deposit_amount,
        "Should have refunded the excess deposit, but got {}",
        balance.data.as_ref().unwrap().total
    );
}

#[tokio::test]
async fn test_force_unregister_integration() {
    let ctx = setup_ft_contract().await;

    let deposit_amount = NearToken::from_near(10);
    ctx.contract
        .storage_deposit()
        .deposit(ctx.account.clone(), deposit_amount)
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    ctx.contract
        .call_function("near_deposit", ())
        .transaction()
        .deposit(NearToken::from_yoctonear(1000))
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();

    // Should fail because we have a balance
    ctx.contract
        .storage_deposit()
        .unregister()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_failure();

    ctx.contract
        .storage_deposit()
        .unregister()
        .force()
        .with_signer(ctx.account.clone(), ctx.signer.clone())
        .send_to(&ctx.network)
        .await
        .unwrap()
        .assert_success();
}
