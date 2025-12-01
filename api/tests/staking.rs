use near_api::{AccountId, NearToken, RPCEndpoint, Signer, Staking};
use near_sandbox::{
    config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY, sandbox::patch::StateRecord, FetchData,
};
use testresult::TestResult;

struct TestContext {
    _sandbox: near_sandbox::Sandbox,
    network: near_api::NetworkConfig,
    signer: std::sync::Arc<Signer>,
    staker: AccountId,
    staking_pool: AccountId,
}

async fn init() -> Result<TestContext, testresult::TestError> {
    let staker: AccountId = "dev.near".parse()?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    sandbox.create_account(staker.clone()).send().await?;

    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    // Set-up staking pool.
    let staking_pool: AccountId = "qbit.poolv1.near".parse()?;
    let mut patch = sandbox
        .patch_state(staking_pool.clone())
        .with_default_access_key()
        .fetch_from(RPCEndpoint::mainnet().url, FetchData::NONE.account().code())
        .await?;

    // Set locked as zero, so we can initialize the pool
    patch.state.iter_mut().for_each(|e| {
        if let StateRecord::Account { account, .. } = e {
            account["locked"] = "0".into();
        }
    });

    patch.send().await?;

    // Init staking pool
    near_api::Contract(staking_pool.clone())
        .call_function(
            "new",
            serde_json::json!({
                "owner_id": staking_pool,
                "stake_public_key": "KuTCtARNzxZQ3YvXDeLjx83FDqxv2SdQTSbiq876zR7",
                "reward_fee_fraction": {
                    "numerator": 0,
                    "denominator": 100,
                }
            }),
        )
        .transaction()
        .with_signer(staking_pool.clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    Ok(TestContext {
        _sandbox: sandbox,
        network,
        signer,
        staker,
        staking_pool,
    })
}

#[tokio::test]
async fn test_user_can_deposit_balance_reflected() -> TestResult {
    let ctx = init().await?;
    let staker_delegation = Staking::delegation(ctx.staker.clone());

    staker_delegation
        .deposit(ctx.staking_pool.clone(), NearToken::from_near(5))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(ctx.staking_pool.clone())
        .fetch_from(&ctx.network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(0));
    assert_eq!(balance.unstaked, NearToken::from_near(5));
    assert_eq!(balance.total, NearToken::from_near(5));

    Ok(())
}

#[tokio::test]
async fn test_user_can_deposit_and_withdraw() -> TestResult {
    let ctx = init().await?;
    let staker_delegation = Staking::delegation(ctx.staker.clone());

    staker_delegation
        .deposit(ctx.staking_pool.clone(), NearToken::from_near(5))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    staker_delegation
        .withdraw(ctx.staking_pool.clone(), NearToken::from_near(2))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(ctx.staking_pool.clone())
        .fetch_from(&ctx.network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(0));
    assert_eq!(balance.unstaked, NearToken::from_near(3));
    assert_eq!(balance.total, NearToken::from_near(3));

    Ok(())
}

#[tokio::test]
async fn test_user_can_deposit_and_stake_two_calls() -> TestResult {
    let ctx = init().await?;
    let staker_delegation = Staking::delegation(ctx.staker.clone());

    staker_delegation
        .deposit(ctx.staking_pool.clone(), NearToken::from_near(5))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    staker_delegation
        .stake(ctx.staking_pool.clone(), NearToken::from_near(3))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(ctx.staking_pool.clone())
        .fetch_from(&ctx.network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(3));
    assert_eq!(balance.unstaked, NearToken::from_near(2));
    assert_eq!(balance.total, NearToken::from_near(5));

    Ok(())
}

#[tokio::test]
async fn test_user_can_deposit_and_stake_single_call() -> TestResult {
    let ctx = init().await?;
    let staker_delegation = Staking::delegation(ctx.staker.clone());

    staker_delegation
        .deposit_and_stake(ctx.staking_pool.clone(), NearToken::from_near(5))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(ctx.staking_pool.clone())
        .fetch_from(&ctx.network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(5));
    assert_eq!(balance.unstaked, NearToken::from_near(0));
    assert_eq!(balance.total, NearToken::from_near(5));

    Ok(())
}

#[tokio::test]
async fn test_user_can_unstake_but_cannot_withdraw_immediately() -> TestResult {
    let ctx = init().await?;
    let staker_delegation = Staking::delegation(ctx.staker.clone());

    staker_delegation
        .deposit_and_stake(ctx.staking_pool.clone(), NearToken::from_near(5))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    staker_delegation
        .unstake(ctx.staking_pool.clone(), NearToken::from_near(3))
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(ctx.staking_pool.clone())
        .fetch_from(&ctx.network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(2));
    assert_eq!(balance.unstaked, NearToken::from_near(3));
    assert_eq!(balance.total, NearToken::from_near(5));

    // Can't withdraw immediately after unstaking due to minimum withdrawal period
    assert!(staker_delegation
        .withdraw_all(ctx.staking_pool.clone())
        .with_signer(ctx.signer.clone())
        .send_to(&ctx.network)
        .await?
        .is_failure());

    Ok(())
}
