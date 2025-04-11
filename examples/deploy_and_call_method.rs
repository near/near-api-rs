use near_api::*;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let network = &sandbox.network_config;

    let signer = Signer::new(Signer::from_secret_key(account_sk)).unwrap();

    // Let's deploy the contract. The contract is simple counter with `get_num`, `increase`, `decrease` arguments
    Contract::deploy(
        account_id.clone(),
        include_bytes!("../resources/counter.wasm").to_vec(),
    )
    // You can add init call as well using `with_init_call`
    .without_init_call()
    .with_signer(signer.clone())
    .send_to(&network)
    .await
    .unwrap();

    let contract = Contract(account_id.clone());

    // Let's fetch current value on a contract
    let current_value: Data<i8> = contract
        // Please note that you can add any argument as long as it is deserializable by serde :)
        // feel free to use serde_json::json macro as well
        .call_function("get_num", ())
        .unwrap()
        .read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Current value: {}", current_value.data);

    // Here is a transaction that require signing compared to view call that was used before.
    contract
        .call_function("increment", ())
        .unwrap()
        .transaction()
        .with_signer(account_id.clone(), signer.clone())
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

    println!("Current value: {}", current_value.data);
}
