use near_api::*;

use near_primitives::hash::hash;

#[tokio::test]
async fn deploy_global_contract_as_account_id_and_use_it() {
    let network = near_workspaces::sandbox().await.unwrap();
    let global_contract = network.dev_create_account().await.unwrap();
    let contract_acc = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    Contract::deploy_global_contract_code(include_bytes!("../resources/counter.wasm").to_vec())
        .as_account_id(global_contract.id().clone())
        .with_signer(Signer::new(Signer::from_workspace(&global_contract)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(contract_acc.id().clone())
        .use_global_account_id(global_contract.id().clone())
        .without_init_call()
        .with_signer(Signer::new(Signer::from_workspace(&contract_acc)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(contract_acc.id().clone());

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
            contract_acc.id().clone(),
            Signer::new(Signer::from_workspace(&contract_acc)).unwrap(),
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
async fn deploy_global_contract_as_hash_and_use_it() {
    let network = near_workspaces::sandbox().await.unwrap();
    let contract_acc = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let code = include_bytes!("../resources/counter.wasm").to_vec();
    let hash = hash(&code);

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(
            contract_acc.id().clone(),
            Signer::new(Signer::from_workspace(&contract_acc)).unwrap(),
        )
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(contract_acc.id().clone())
        .use_global_hash(hash.into())
        .without_init_call()
        .with_signer(Signer::new(Signer::from_workspace(&contract_acc)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(contract_acc.id().clone());

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
            contract_acc.id().clone(),
            Signer::new(Signer::from_workspace(&contract_acc)).unwrap(),
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
