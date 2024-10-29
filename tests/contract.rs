use near_api::prelude::*;

use serde_json::json;

#[tokio::test]
async fn contract_without_init_call() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let contract = Contract(account.id().clone());

    contract
        .deploy(include_bytes!("../resources/counter.wasm").to_vec())
        .without_init_call()
        .with_signer(Signer::new(Signer::from_workspace(&account)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    assert!(!contract
        .wasm()
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .code
        .is_empty());

    assert!(contract
        .contract_source_metadata()
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .version
        .is_some());

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .unwrap()
        .read_only()
        .fetch_from(&network)
        .await
        .unwrap();
    assert_eq!(current_value.data, 0);

    contract
        .call_function("increment", ())
        .unwrap()
        .transaction()
        .with_signer(
            account.id().clone(),
            Signer::new(Signer::from_workspace(&account)).unwrap(),
        )
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .unwrap()
        .read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(current_value.data, 1);
}

#[tokio::test]
async fn contract_with_init_call() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let contract = Contract(account.id().clone());

    contract
        .deploy(include_bytes!("../resources/fungible_token.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": account.id().to_string(),
                "total_supply": "1000000000000000000000000000"
            }),
        )
        .unwrap()
        .with_signer(Signer::new(Signer::from_workspace(&account)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    assert!(!contract
        .wasm()
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .code
        .is_empty());
}
