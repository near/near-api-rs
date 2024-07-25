use near::{signer::Signer, types::tokens::FTBalance, Account, Contract, NetworkConfig, Tokens};
use near_sdk::NearToken;
use serde_json::json;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let token = network.dev_create_account().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    // Let's deploy the contract. The contract is simple counter with `get_num`, `increase`, `decrease` arguments
    Contract(token.id().clone())
        .deploy(include_bytes!("./resources/fungible_token.wasm").to_vec())
        // You can add init call as well using `with_init_call`
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

    let tokens = Tokens::of(token.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {} tokens", tokens.to_whole());

    Account(token.id().clone())
        .storage(token.id().clone())
        .deposit(account.id().clone(), NearToken::from_millinear(100))
        .unwrap()
        .with_signer(Signer::from_workspace(&token))
        .send_to(&network)
        .await
        .unwrap();

    Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        // Send 100 tokens with 24 decimals (default for FT)
        .ft(token.id().clone(), FTBalance::from_whole(100, 24))
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

    // We validate decimals at the network level
    let token = Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        .ft(token.id().clone(), FTBalance::from_whole(1, 8))
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
