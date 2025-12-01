use near_api::{
    signer::generate_secret_key,
    types::{AccountId, NearToken},
    Account, NetworkConfig, Signer, Tokens,
};
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use testresult::TestResult;

#[tokio::main]
async fn main() -> TestResult {
    let network = near_sandbox::Sandbox::start_sandbox().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", network.rpc_addr.parse()?);
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let balance = Tokens::account(account.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;

    println!("Balance: {}", balance.total);

    let new_account: AccountId = format!("{}.{}", "bob", account).parse()?;

    Account::create_account(new_account.clone())
        .fund_myself(account.clone(), NearToken::from_near(1))
        .public_key(generate_secret_key()?.public_key())?
        .with_signer(signer.clone())
        .send_to(&network)
        .await?
        .assert_success();

    Tokens::account(account.clone())
        .send_to(new_account.clone())
        .near(NearToken::from_near(1))
        .with_signer(signer)
        .send_to(&network)
        .await?
        .assert_success();

    let new_account_balance = Tokens::account(account.clone())
        .near_balance()
        .fetch_from(&network)
        .await?;
    let bob_balance = Tokens::account(new_account)
        .near_balance()
        .fetch_from(&network)
        .await?;

    println!("Balance: {}", new_account_balance.total);
    // Expect to see 2 NEAR in Bob's account. 1 NEAR from create_account and 1 NEAR from send_near
    println!("Bob balance: {}", bob_balance.total);

    Ok(())
}
