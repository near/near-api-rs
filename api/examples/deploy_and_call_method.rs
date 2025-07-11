use near_api::{
    types::{AccountId, Data},
    *,
};
use near_sandbox_utils::high_level::config::DEFAULT_GENESIS_ACCOUNT;

#[tokio::main]
async fn main() {
    let network = near_sandbox_utils::high_level::Sandbox::start_sandbox()
        .await
        .unwrap();
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    let signer = Signer::new(Signer::default_sandbox()).unwrap();

    // Let's deploy the contract. The contract is simple counter with `get_num`, `increase`, `decrease` arguments
    Contract::deploy(account.clone())
        .use_code(include_bytes!("../resources/counter.wasm").to_vec())
        // You can add init call as well using `with_init_call`
        .without_init_call()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    let contract = Contract(account.clone());

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
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    let current_value: Data<i8> = contract
        .call_function("get_num", ())
        .unwrap()
        .read_only()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Current value: {}", current_value.data);
}
