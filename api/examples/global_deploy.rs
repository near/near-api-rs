use near_api::{types::CryptoHash, Contract, NetworkConfig, Signer};
use near_sandbox::{GenesisAccount, SandboxConfig};

#[tokio::main]
async fn main() {
    let global = GenesisAccount::generate_with_name("global".parse().unwrap());
    let instance_of_global =
        GenesisAccount::generate_with_name("instance_of_global".parse().unwrap());
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config_and_version(
        SandboxConfig {
            additional_accounts: vec![global.clone(), instance_of_global.clone()],
            ..Default::default()
        },
        "2.9.0",
    )
    .await
    .unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    let global_signer = Signer::new(Signer::from_secret_key(
        global.private_key.clone().parse().unwrap(),
    ))
    .unwrap();
    let instance_of_global_signer = Signer::new(Signer::from_secret_key(
        instance_of_global.private_key.clone().parse().unwrap(),
    ))
    .unwrap();

    let code: Vec<u8> = include_bytes!("../resources/counter.wasm").to_vec();
    let contract_hash = CryptoHash::hash(&code);

    // Publish contract code as immutable hash
    Contract::publish_contract(code.clone(), None)
        .from_signer_account()
        .with_signer(global.account_id.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Publish contract code as mutable account ID
    Contract::publish_contract(code, Some(global.account_id.clone()))
        .from_signer_account()
        .with_signer(global.account_id.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Deploy from published code using account ID reference
    Contract::deploy(instance_of_global.account_id.clone())
        .deploy_from_published(global.account_id.clone())
        .without_init_call()
        .with_signer(instance_of_global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Deploy from published code using hash reference
    Contract::deploy(instance_of_global.account_id.clone())
        .deploy_from_published(contract_hash)
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
