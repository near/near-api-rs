use near_api::*;

use near_token::NearToken;
use signer::generate_secret_key;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let network = &sandbox.network_config;

    let balance = Tokens::account(account_id.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Balance: {}", balance.total);

    let new_account: AccountId = format!("{}.{}", "bob", account_id).parse().unwrap();
    let signer = Signer::new(Signer::from_secret_key(account_sk)).unwrap();

    Account::create_account(new_account.clone())
        .fund_myself(account_id.clone(), NearToken::from_near(1))
        .public_key(generate_secret_key().unwrap().public_key())
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    Tokens::account(account_id.clone())
        .send_to(new_account.clone())
        .near(NearToken::from_near(1))
        .with_signer(signer)
        .send_to(&network)
        .await
        .unwrap();

    let new_account_balance = Tokens::account(account_id.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();
    let bob_balance = Tokens::account(new_account)
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Balance: {}", new_account_balance.total);
    // Expect to see 2 NEAR in Bob's account. 1 NEAR from create_account and 1 NEAR from send_near
    println!("Bob balance: {}", bob_balance.total);
}
