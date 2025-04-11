use near_api::*;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let target_account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let signer = Signer::new(Signer::from_workspace(&account)).unwrap();

    let deploy_hash_result =
        Contract::deploy_global_contract_code(include_bytes!("../resources/counter.wasm").to_vec())
            .as_hash(account.id().clone())
            .with_signer(signer.clone())
            .send_to(&network)
            .await
            .unwrap()
            .assert_success();

    Contract::deploy_global_contract_code(include_bytes!("../resources/counter.wasm").to_vec())
        .as_account_id(account.id().clone())
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(target_account.id().clone())
        .with_global_account_id(account.id().clone())
        .without_init_call()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    Contract::deploy(target_account.id().clone())
        .with_global_hash(todo!("Use hash"))
        .without_init_call()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    println!(
        "Successfully deployed contract using both global hash and global account ID methods!"
    );
}
