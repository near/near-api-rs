use std::sync::Arc;

use near_api::{AccountId, Contract, NetworkConfig, RPCEndpoint, Signer};
use near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;
use testresult::TestError;

pub struct TestContext {
    #[allow(dead_code)]
    pub sandbox: near_sandbox::Sandbox,
    pub network: NetworkConfig,
    pub account: AccountId,
    pub contract: Contract,
    pub signer: Arc<Signer>,
}

pub async fn setup_social_contract() -> Result<TestContext, TestError> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let contract = Contract("social.near".parse()?);
    let account: AccountId = "user.sandbox".parse()?;

    sandbox.create_account(account.clone()).send().await?;

    sandbox
        .import_account(RPCEndpoint::mainnet().url, contract.account_id().clone())
        .send()
        .await?;

    contract
        .call_function("new", ())
        .transaction()
        .with_signer(contract.account_id().clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    contract
        .call_function("set_status", serde_json::json!({ "status": "Live" }))
        .transaction()
        .with_signer(contract.account_id().clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    Ok(TestContext {
        contract,
        sandbox,
        network,
        account,
        signer,
    })
}

pub async fn setup_ft_contract() -> Result<TestContext, TestError> {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let contract = Contract("wrap.near".parse()?);
    let account: AccountId = "user.sandbox".parse()?;

    sandbox.create_account(account.clone()).send().await?;

    sandbox
        .import_account(RPCEndpoint::mainnet().url, contract.account_id().clone())
        .send()
        .await?;

    contract
        .call_function("new", ())
        .transaction()
        .with_signer(contract.account_id().clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    Ok(TestContext {
        contract,
        sandbox,
        network,
        account,
        signer,
    })
}
