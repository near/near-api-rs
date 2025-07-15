use near_api::{
    types::{AccountId, CryptoHash},
    *,
};
use near_sandbox_utils::{GenesisAccount, SandboxConfig};

#[tokio::main]
async fn main() {
    let global: AccountId = "global.testnet".parse().unwrap();
    let instance_of_global: AccountId = "instance_of_global.testnet".parse().unwrap();
    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![
                GenesisAccount {
                    account_id: global.to_string(),
                    ..Default::default()
                },
                GenesisAccount {
                    account_id: instance_of_global.to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    let global_signer = Signer::new(Signer::default_sandbox()).unwrap();
    let instance_of_global_signer = Signer::new(Signer::default_sandbox()).unwrap();

    let code: Vec<u8> = include_bytes!("../resources/counter.wasm").to_vec();
    let contract_hash = CryptoHash::hash(&code);

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(global.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy_global_contract_code(code)
        .as_account_id(global.clone())
        .with_signer(global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.clone())
        .use_global_account_id(global.clone())
        .without_init_call()
        .with_signer(instance_of_global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.clone())
        .use_global_hash(contract_hash)
        .without_init_call()
        .with_signer(instance_of_global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    println!(
        "Successfully deployed contract using both global hash and global account ID methods!"
    );
}
