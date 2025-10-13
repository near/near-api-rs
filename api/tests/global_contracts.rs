use near_api::*;

use near_api_types::{AccountId, CryptoHash, Data};
use near_sandbox::{
    GenesisAccount, SandboxConfig,
    config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY},
};

#[tokio::test]
async fn deploy_global_contract_as_account_id_and_use_it() {
    let global_contract = GenesisAccount::generate_with_name("global_contract".parse().unwrap());
    let account_signer = Signer::new(Signer::from_secret_key(
        global_contract.private_key.parse().unwrap(),
    ))
    .unwrap();

    let global_signer = Signer::new(Signer::from_secret_key(
        global_contract.private_key.parse().unwrap(),
    ))
    .unwrap();

    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![global_contract.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    Contract::deploy_global_contract_code(include_bytes!("../resources/counter.wasm").to_vec())
        .as_account_id(global_contract.account_id.clone())
        .with_signer(global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(global_contract.account_id.clone())
        .use_global_account_id(global_contract.account_id.clone())
        .without_init_call()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(global_contract.account_id.clone());

    assert!(
        !contract
            .wasm()
            .fetch_from(&network)
            .await
            .unwrap()
            .data
            .code_base64
            .is_empty()
    );

    assert!(
        contract
            .contract_source_metadata()
            .fetch_from(&network)
            .await
            .unwrap()
            .data
            .version
            .is_some()
    );

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
        .with_signer(global_contract.account_id.clone(), account_signer.clone())
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
    let global_contract = GenesisAccount::generate_with_name("global_contract".parse().unwrap());
    let account_signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();
    let global_signer = Signer::new(Signer::from_secret_key(
        global_contract.private_key.parse().unwrap(),
    ))
    .unwrap();
    let account_id: AccountId = DEFAULT_GENESIS_ACCOUNT.into();

    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![global_contract.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    let code = include_bytes!("../resources/counter.wasm").to_vec();
    let hash = CryptoHash::hash(&code);

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(global_contract.account_id.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(account_id.clone())
        .use_global_hash(hash)
        .without_init_call()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(account_id.clone());

    assert!(
        !contract
            .wasm()
            .fetch_from(&network)
            .await
            .unwrap()
            .data
            .code_base64
            .is_empty()
    );

    assert!(
        contract
            .contract_source_metadata()
            .fetch_from(&network)
            .await
            .unwrap()
            .data
            .version
            .is_some()
    );

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
        .with_signer(account_id.clone(), account_signer.clone())
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
