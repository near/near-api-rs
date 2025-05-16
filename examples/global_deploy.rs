use near_api::*;
use near_primitives::hash::hash;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let global = network.dev_create_account().await.unwrap();
    let instance_of_global = network.dev_create_account().await.unwrap();

    let network = NetworkConfig::from(network);

    let global_signer = Signer::new(Signer::from_workspace(&global)).unwrap();
    let instance_of_global_signer =
        Signer::new(Signer::from_workspace(&instance_of_global)).unwrap();

    let code: Vec<u8> = include_bytes!("../resources/counter.wasm").to_vec();
    let contract_hash = hash(&code);

    Contract::deploy_global_contract_code(code.clone())
        .as_hash()
        .with_signer(global.id().clone(), global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy_global_contract_code(code)
        .as_account_id(global.id().clone())
        .with_signer(global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.id().clone())
        .use_global_account_id(global.id().clone())
        .without_init_call()
        .with_signer(instance_of_global_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(instance_of_global.id().clone())
        .use_global_hash(contract_hash.into())
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
