use near_api::{
    AccountId, Contract, NearToken, NetworkConfig, RPCEndpoint, Signer, StorageDeposit,
};
use near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;

#[tokio::main]
async fn main() -> testresult::TestResult {
    let account: AccountId = "dev.near".parse()?;
    let token: AccountId = "wrap.near".parse()?;

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    sandbox.create_account(account.clone()).send().await?;
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    // Import wNEAR contract from mainnet
    sandbox
        .import_account(RPCEndpoint::mainnet().url, token.clone())
        .send()
        .await?;

    Contract(token.clone())
        .call_function("new", ())
        .transaction()
        .with_signer(token.clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let storage = StorageDeposit::on_contract(token.clone());

    // Check storage balance (None for unregistered account)
    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await?;
    assert!(balance.data.is_none());

    // Deposit storage
    storage
        .deposit(account.clone(), NearToken::from_millinear(100))
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    // Verify storage balance
    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await?
        .data
        .ok_or("Balance is none")?;

    assert!(balance.total.as_millinear() > 0);

    storage
        .unregister()
        .with_signer(account.clone(), signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    let balance = storage
        .view_account_storage(account.clone())
        .fetch_from(&network)
        .await?;
    assert!(balance.data.is_none());

    Ok(())
}
