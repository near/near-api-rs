use std::sync::Arc;

use near_api::{AccountId, NearToken, NetworkConfig, RPCEndpoint, Signer, Staking};
use near_sandbox::{
    config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY, sandbox::patch::StateRecord, FetchData,
};

#[tokio::main]
async fn main() -> testresult::TestResult {
    let staker: AccountId = "dev.near".parse()?;
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    sandbox.create_account(staker.clone()).send().await?;
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let staking_pool = setup_staking_pool(&sandbox, &network, signer.clone()).await?;

    let staker_delegation = Staking::delegation(staker.clone());
    staker_delegation
        .deposit(staking_pool.clone(), NearToken::from_near(5))
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    staker_delegation
        .withdraw(staking_pool.clone(), NearToken::from_near(2))
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    staker_delegation
        .stake(staking_pool.clone(), NearToken::from_near(1))
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .fetch_from(&network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(1));
    assert_eq!(balance.unstaked, NearToken::from_near(2));
    assert_eq!(balance.total, NearToken::from_near(3));

    staker_delegation
        .deposit_and_stake(staking_pool.clone(), NearToken::from_near(1))
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .fetch_from(&network)
        .await?;

    assert_eq!(balance.staked, NearToken::from_near(2));
    assert_eq!(balance.unstaked, NearToken::from_near(2));
    assert_eq!(balance.total, NearToken::from_near(4));

    staker_delegation
        .unstake(staking_pool.clone(), NearToken::from_near(1))
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .fetch_from(&network)
        .await?;
    assert_eq!(balance.staked, NearToken::from_near(1));
    assert_eq!(balance.unstaked, NearToken::from_near(3));
    assert_eq!(balance.total, NearToken::from_near(4));

    // Can't be withdrawn yet as it should pass the minimum withdrawal period
    staker_delegation
        .withdraw_all(staking_pool.clone())
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_failure();

    Ok(())
}

async fn setup_staking_pool(
    sandbox: &near_sandbox::Sandbox,
    network: &NetworkConfig,
    signer: Arc<Signer>,
) -> Result<AccountId, testresult::TestError> {
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
        .with_signer(staking_pool.clone(), signer)
        .send_to(network)
        .await?
        .assert_success();

    Ok(staking_pool)
}
