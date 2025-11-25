use std::sync::Arc;

use near_api::{AccountId, Contract, NetworkConfig, RPCEndpoint, Signer};
use near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;

pub struct TestContext {
    #[allow(dead_code)]
    pub sandbox: near_sandbox::Sandbox,
    pub network: NetworkConfig,
    pub account: AccountId,
    pub contract: Contract,
    pub signer: Arc<Signer>,
}

pub async fn setup_social_contract() -> TestContext {
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
        contract,
        sandbox,
        network,
        account,
        signer,
    }
}

pub async fn setup_ft_contract() -> TestContext {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    let contract = Contract("wrap.near".parse().unwrap());
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

    TestContext {
        contract,
        sandbox,
        network,
        account,
        signer,
    }
}
