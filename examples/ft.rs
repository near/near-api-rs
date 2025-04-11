use near_api::*;

use serde_json::json;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();

    let token_account_id = "token.sandbox".parse().unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let token_sk = sandbox
        .create_root_subaccount(&token_account_id)
        .await
        .unwrap();
    let _account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let token_signer = Signer::new(Signer::from_secret_key(token_sk)).unwrap();
    let network = &sandbox.network_config;

    // Deploying token contract
    Contract::deploy(
        token_account_id.clone(),
        include_bytes!("../resources/fungible_token.wasm").to_vec(),
    )
    .with_init_call(
        "new_default_meta",
        json!({
            "owner_id": &token_account_id,
            "total_supply": "1000000000000000000000000000"
        }),
    )
    .unwrap()
    .with_signer(token_signer.clone())
    .send_to(network)
    .await
    .unwrap();

    // Verifying that user has 1000 tokens
    let tokens = Tokens::account(token_account_id.clone())
        .ft_balance(token_account_id.clone())
        .unwrap()
        .fetch_from(network)
        .await
        .unwrap();

    println!("Owner has {}", tokens);

    // Transfer 100 tokens to the account
    // We handle internally the storage deposit for the receiver account
    Tokens::account(token_account_id.clone())
        .send_to(account_id.clone())
        .ft(
            token_account_id.clone(),
            // Send 1.5 tokens
            FTBalance::with_decimals(24).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(token_signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let tokens = Tokens::account(account_id.clone())
        .ft_balance(token_account_id.clone())
        .unwrap()
        .fetch_from(network)
        .await
        .unwrap();

    println!("Account has {}", tokens);

    let tokens = Tokens::account(token_account_id.clone())
        .ft_balance(token_account_id.clone())
        .unwrap()
        .fetch_from(network)
        .await
        .unwrap();

    println!("Owner has {}", tokens);

    // We validate decimals at the network level so this should fail with a validation error
    let token = Tokens::account(token_account_id.clone())
        .send_to(account_id.clone())
        .ft(
            token_account_id.clone(),
            FTBalance::with_decimals(8).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(token_signer)
        .send_to(network)
        .await;

    assert!(token.is_err());
    println!(
        "Expected decimal validation error: {}",
        token.err().unwrap()
    );
}
