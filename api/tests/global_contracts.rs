use near_api::*;

use near_sandbox_utils::{
    GenesisAccount, SandboxConfig, high_level::config::DEFAULT_GENESIS_ACCOUNT,
};
use near_types::{AccountId, Data, hash};

#[tokio::test]
async fn deploy_global_contract_as_account_id_and_use_it() {
    let global_contract_id: AccountId = "global_contract.testnet".parse().unwrap();
    let account_id: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let account_signer = Signer::new(Signer::default_sandbox()).unwrap();
    let global_signer = Signer::new(Signer::default_sandbox()).unwrap();

    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: global_contract_id.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    Contract::deploy_global_contract_code(include_bytes!("../resources/counter.wasm").to_vec())
        .as_account_id(global_contract_id.clone())
        .with_signer(global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    Contract::deploy(account_id.clone())
        .use_global_account_id(global_contract_id.clone())
        .without_init_call()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

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
        .into_result()
        .unwrap();

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
    let global_contract_id: AccountId = "global_contract.testnet".parse().unwrap();
    let account_id: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let account_signer = Signer::new(Signer::default_sandbox()).unwrap();
    let global_signer = Signer::new(Signer::default_sandbox()).unwrap();

    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: global_contract_id.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    let code = include_bytes!("../resources/counter.wasm").to_vec();
    let hash = hash(&code);

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(global_contract_id.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    Contract::deploy(account_id.clone())
        .use_global_hash(hash)
        .without_init_call()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

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
        .into_result()
        .unwrap();

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .unwrap()
        .read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(current_value.data, 1);
}
