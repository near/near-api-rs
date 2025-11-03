use near_api::{types::AccessKey, AccountId, NearToken, Signer, Staking};
use near_sandbox::{config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY, GenesisAccount, SandboxConfig};

#[tokio::main]
async fn main() {
    let staker: AccountId = "yurtur.near".parse().unwrap();
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![GenesisAccount::default_with_name(staker.clone()).clone()],
        ..SandboxConfig::default()
    })
    .await
    .unwrap();
    let network =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    // Set-up staking pool.
    let staking_pool: AccountId = "qbit.poolv1.near".parse().unwrap();
    let mut pool_account = near_api::Account(staking_pool.clone())
        .view()
        .fetch_from_mainnet()
        .await
        .unwrap()
        .data;
    pool_account.locked = NearToken::from_near(0);
    let pool_code = near_api::Contract(staking_pool.clone())
        .wasm()
        .fetch_from_mainnet()
        .await
        .unwrap()
        .data
        .code_base64;
    sandbox
        .patch_state(staking_pool.clone())
        .account(pool_account)
        .access_key(
            signer.get_public_key().await.unwrap().to_string(),
            AccessKey {
                nonce: 0.into(),
                permission: near_api::types::AccessKeyPermission::FullAccess,
            },
        )
        .code(pool_code)
        .send()
        .await
        .unwrap();

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
        .unwrap()
        .transaction()
        .with_signer(staking_pool.clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let staker_delegation = Staking::delegation(staker.clone());
    staker_delegation
        .deposit(staking_pool.clone(), NearToken::from_near(5))
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    staker_delegation
        .withdraw(staking_pool.clone(), NearToken::from_near(2))
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    staker_delegation
        .stake(staking_pool.clone(), NearToken::from_near(1))
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(balance.staked, NearToken::from_near(1));
    assert_eq!(balance.unstaked, NearToken::from_near(2));
    assert_eq!(balance.total, NearToken::from_near(3));

    staker_delegation
        .deposit_and_stake(staking_pool.clone(), NearToken::from_near(1))
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(balance.staked, NearToken::from_near(2));
    assert_eq!(balance.unstaked, NearToken::from_near(2));
    assert_eq!(balance.total, NearToken::from_near(4));

    staker_delegation
        .unstake(staking_pool.clone(), NearToken::from_near(1))
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let balance = staker_delegation
        .view_balance(staking_pool.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();
    assert_eq!(balance.staked, NearToken::from_near(1));
    assert_eq!(balance.unstaked, NearToken::from_near(3));
    assert_eq!(balance.total, NearToken::from_near(4));

    // Can't be withdrawn yet as it should pass the minimum withdrawal period
    assert!(staker_delegation
        .withdraw_all(staking_pool.clone())
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .is_failure());
}
