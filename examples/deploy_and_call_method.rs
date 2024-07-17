use near::{signer::Signer, types::Data, Contract, NetworkConfig};

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    Contract(account.id().clone())
        .deploy(include_bytes!("./resources/counter.wasm").to_vec())
        .without_init_call()
        .with_signer(Signer::from_workspace(&account))
        .send_to(&network)
        .await
        .unwrap();

    let current_value: Data<i8> = Contract(account.id().clone())
        .call_function("get_num", ())
        .unwrap()
        .as_read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Current value: {}", current_value.data);

    Contract(account.id().clone())
        .call_function("increment", ())
        .unwrap()
        .as_transaction()
        .with_signer(account.id().clone(), Signer::from_workspace(&account))
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let current_value: Data<i8> = Contract(account.id().clone())
        .call_function("get_num", ())
        .unwrap()
        .as_read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Current value: {}", current_value.data);
}
