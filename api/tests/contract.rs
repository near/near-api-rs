use near_api::*;

use near_api_types::{AccountId, Data};
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use serde_json::json;
use testresult::TestResult;

#[tokio::test]
async fn contract_without_init_call() -> TestResult {
    let network = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", network.rpc_addr.parse()?);
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    Contract::deploy(account.clone())
        .use_code(include_bytes!("../resources/counter.wasm").to_vec())
        .without_init_call()
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let contract = Contract(account.clone());

    assert!(!contract
        .wasm()
        .fetch_from(&network)
        .await?
        .data
        .code_base64
        .is_empty());

    assert!(contract
        .contract_source_metadata()
        .fetch_from(&network)
        .await?
        .data
        .version
        .is_some());

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .read_only()
        .fetch_from(&network)
        .await?;
    assert_eq!(current_value.data, 0);

    contract
        .call_function("increment", ())
        .transaction()
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .read_only()
        .fetch_from(&network)
        .await?;

    assert_eq!(current_value.data, 1);

    Ok(())
}

#[tokio::test]
async fn contract_with_init_call() -> TestResult {
    let network = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", network.rpc_addr.parse()?);
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?,
    ))?;

    Contract::deploy(account.clone())
        .use_code(include_bytes!("../resources/fungible_token.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": account,
                "total_supply": "1000000000000000000000000000"
            }),
        )?
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let contract = Contract(account.clone());

    assert!(!contract
        .wasm()
        .fetch_from(&network)
        .await?
        .data
        .code_base64
        .is_empty());

    Ok(())
}
