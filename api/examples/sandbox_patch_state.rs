use near_api::{AccountId, Chain, NetworkConfig, types::sandbox::StateRecord};

#[tokio::main]
async fn main() {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
    let account_id: AccountId = "usdt.tether-token.near".parse().unwrap();

    // Fetched account from the mainnet
    let account = near_api::Account(account_id.clone())
        .view()
        .fetch_from_mainnet()
        .await
        .unwrap()
        .data;

    // Try to fetch the account from the sandbox
    near_api::Account(account_id.clone())
        .view()
        .fetch_from(&network)
        .await
        .expect_err("Account should not be found in the sandbox");

    let state_changes = vec![StateRecord::Account {
        account_id: account_id.clone(),
        account: account.clone(),
    }];

    near_api::Sandbox::patch_state(state_changes)
        .fetch_from(&network)
        .await
        .unwrap();

    // Try to fetch the account from the sandbox
    let moved_account = near_api::Account(account_id.clone())
        .view()
        .fetch_from(&network)
        .await
        .unwrap()
        .data;

    assert_eq!(moved_account.amount, account.amount);
    assert_eq!(moved_account.contract_state, account.contract_state);

    let block_height = Chain::block_number().fetch_from(&network).await.unwrap();

    near_api::Sandbox::fast_forward(1000)
        .fetch_from(&network)
        .await
        .unwrap();

    let block_height_after = Chain::block_number().fetch_from(&network).await.unwrap();
    assert!(
        block_height_after >= block_height + 100,
        "Block height should be {block_height_after} >= {block_height} + 100"
    );
}
