use near_api::{AccountId, Chain, Contract, NetworkConfig};
use serde_json::json;

#[tokio::main]
async fn main() {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let sandbox_network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let account_id: AccountId = "race-of-sloths.testnet".parse().unwrap();

    let account = near_api::Account(account_id.clone())
        .view()
        .fetch_from(&NetworkConfig::testnet())
        .await
        .unwrap()
        .data;

    // Try to fetch the account from the sandbox
    near_api::Account(account_id.clone())
        .view()
        .fetch_from(&sandbox_network)
        .await
        .expect_err("Account should not be found in the sandbox");

    let _signer = near_api::Sandbox::import_account(account_id.clone())
        .import_state()
        .source_network(NetworkConfig::testnet())
        .post_to(&sandbox_network)
        .await
        .unwrap();

    let moved_account = near_api::Account(account_id.clone())
        .view()
        .fetch_from(&sandbox_network)
        .await
        .unwrap()
        .data;

    assert_eq!(moved_account.amount, account.amount);
    assert_eq!(moved_account.contract_state, account.contract_state);

    let stats: serde_json::Value = Contract(account_id.clone())
        .call_function(
            "user",
            json!({ "user": "akorchyn", "periods": ["all-time"] }),
        )
        .unwrap()
        .read_only()
        .fetch_from(&sandbox_network)
        .await
        .unwrap()
        .data;

    println!("RoS Testnet Stats: {:#?}", stats);

    let block_height = Chain::block_number()
        .fetch_from(&sandbox_network)
        .await
        .unwrap();

    near_api::Sandbox::fast_forward(1000)
        .fetch_from(&sandbox_network)
        .await
        .unwrap();

    let block_height_after = Chain::block_number()
        .fetch_from(&sandbox_network)
        .await
        .unwrap();
    assert!(
        block_height_after >= block_height + 1000,
        "Block height should be {block_height_after} >= {block_height} + 1000"
    );
}
