use near::{signer::Signer, NetworkConfig};
use near_token::NearToken;
use near_workspaces::AccountId;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let balance = near::Tokens::of(account.id().clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Balance: {}", balance.liquid);

    let new_account: AccountId = format!("{}.{}", "bob", account.id()).parse().unwrap();
    let signer = Signer::new(Signer::from_workspace(&account)).unwrap();

    near::Account::create_account()
        .fund_myself(
            new_account.clone(),
            account.id().clone(),
            NearToken::from_near(1),
        )
        .new_keypair()
        .save_generated_seed_to_file("./new_account_seed".into())
        .unwrap()
        .with_signer(signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    near::Tokens::of(account.id().clone())
        .send_to(new_account.clone())
        .near(NearToken::from_near(1))
        .with_signer(signer)
        .send_to(&network)
        .await
        .unwrap();

    let new_acccount_balance = near::Tokens::of(account.id().clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();
    let bob_balance = near::Tokens::of(new_account)
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Balance: {}", new_acccount_balance.liquid);
    // Expect to see 2 NEAR in Bob's account. 1 NEAR from create_account and 1 NEAR from send_near
    println!("Bob balance: {}", bob_balance.liquid);
}
