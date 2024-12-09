use near_api::*;

use serde_json::json;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let token = network.dev_create_account().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);
    let token_signer = Signer::new(Signer::from_workspace(&token)).unwrap();

    // Deploying token contract
    Contract::deploy(
        token.id().clone(),
        include_bytes!("../resources/fungible_token.wasm").to_vec(),
    )
    .with_init_call(
        "new_default_meta",
        json!({
            "owner_id": token.id().to_string(),
            "total_supply": "1000000000000000000000000000"
        }),
    )
    .unwrap()
    .with_signer(token_signer.clone())
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

    println!("Owner has {}", tokens);

    // Transfer 100 tokens to the account
    // We handle internally the storage deposit for the receiver account
    Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        .ft(
            token.id().clone(),
            // Send 1.5 tokens
            FTBalance::with_decimals(24).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(token_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let tokens = Tokens::of(account.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Account has {}", tokens);

    let tokens = Tokens::of(token.id().clone())
        .ft_balance(token.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {}", tokens);

    // We validate decimals at the network level so this should fail with a validation error
    let token = Tokens::of(token.id().clone())
        .send_to(account.id().clone())
        .ft(
            token.id().clone(),
            FTBalance::with_decimals(8).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(token_signer)
        .send_to(&network)
        .await;

    assert!(token.is_err());
    println!(
        "Expected decimal validation error: {}",
        token.err().unwrap()
    );
}
