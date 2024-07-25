use near::{
    signer::Signer, types::tokens::FTBalance, Contract, NetworkConfig, StorageDeposit, Tokens,
};
use near_sdk::NearToken;
use serde_json::json;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let token = network.dev_create_account().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    // Deploying token contract
    Contract(token.id().clone())
        .deploy(include_bytes!("./resources/fungible_token.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": token.id().to_string(),
                "total_supply": "1000000000000000000000000000"
            }),
        )
        .unwrap()
        .with_signer(Signer::from_workspace(&token))
        .send_to(&network)
        .await
        .unwrap();

    // Verifying that user has 1000 tokens
    let tokens = Tokens::of(token.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {} tokens", tokens.to_whole());

    // Paying for storage for the account.
    // This is required to store the tokens on the account
    // TODO: This should be done automatically by the SDK
    StorageDeposit::on_contract(token.id().clone())
        .deposit(account.id().clone(), NearToken::from_millinear(100))
        .unwrap()
        .with_signer(token.id().clone(), Signer::from_workspace(&token))
        .send_to(&network)
        .await
        .unwrap();

    // Transfer 100 tokens to the account
    Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        .ft(
            token.id().clone(),
            FTBalance::with_decimals(24).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(Signer::from_workspace(&token))
        .send_to(&network)
        .await
        .unwrap();

    let tokens = Tokens::of(account.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Account has {} tokens", tokens.to_whole());

    let tokens = Tokens::of(token.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {} tokens", tokens.to_whole());

    // We validate decimals at the network level so this should fail with a validation error
    let token = Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        .ft(
            token.id().clone(),
            FTBalance::with_decimals(8).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(Signer::from_workspace(&token))
        .send_to(&network)
        .await;

    assert!(token.is_err());
    println!(
        "Expected decimal validation error: {}",
        token.err().unwrap()
    );
}