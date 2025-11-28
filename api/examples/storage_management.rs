use near_api::{
    AccountId, Contract, NearToken, NetworkConfig, RPCEndpoint, Signer, StorageDeposit,
};
use near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;

#[tokio::main]
async fn main() {
    let account: AccountId = "dev.near".parse().unwrap();
    let token: AccountId = "wrap.near".parse().unwrap();

    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    sandbox
        .create_account(account.clone())
        .send()
        .await
        .unwrap();
    let signer = Signer::new(Signer::from_secret_key(
        DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse().unwrap(),
    ))
    .unwrap();

    // Import wNEAR contract from mainnet
    sandbox
        .import_account(RPCEndpoint::mainnet().url, token.clone())
        .send()
        .await
        .unwrap();

    Contract(token.clone())
        .call_function("new", ())
        .transaction()
        .with_signer(token.clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let storage = StorageDeposit::on_contract(token.clone());

    // Check storage balance (None for unregistered account)
    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await
        .unwrap();
    assert!(balance.data.is_none());

    // Deposit storage
    storage
        .deposit(account.clone(), NearToken::from_millinear(100))
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verify storage balance
    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .unwrap();

    assert!(balance.total.as_millinear() > 0);

    storage
        .unregister()
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await
        .unwrap();
    assert!(balance.data.is_none());
}
