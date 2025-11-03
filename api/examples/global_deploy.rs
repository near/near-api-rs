use near_api::{types::CryptoHash, Contract, NetworkConfig, Signer};
use near_sandbox::{GenesisAccount, SandboxConfig};

#[tokio::main]
async fn main() {
    let global = GenesisAccount::generate_with_name("global".parse().unwrap());
    let instance_of_global =
        GenesisAccount::generate_with_name("instance_of_global".parse().unwrap());
    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![global.clone(), instance_of_global.clone()],
        ..Default::default()
    })
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

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(global.account_id.clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy_global_contract_code(code)
        .as_account_id(global.account_id.clone())
        .with_signer(global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.account_id.clone())
        .use_global_account_id(global.account_id.clone())
        .without_init_call()
        .with_signer(instance_of_global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.account_id.clone())
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
