use near_api::{
    types::{AccountId, tokens::FTBalance},
    *,
};
use near_sandbox_utils::{
    GenesisAccount, SandboxConfig, high_level::config::DEFAULT_GENESIS_ACCOUNT,
};

use serde_json::json;

#[tokio::main]
async fn main() {
    let token: AccountId = "token.testnet".parse().unwrap();
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let token_signer = Signer::new(Signer::default_sandbox()).unwrap();

    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: token.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    // Deploying token contract
    Contract::deploy(token.clone())
        .use_code(include_bytes!("../resources/fungible_token.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                    "owner_id": token.to_string(),
                "total_supply": "1000000000000000000000000000"
            }),
        )
        .unwrap()
        .with_signer(token_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verifying that user has 1000 tokens
    let tokens = Tokens::account(token.clone())
        .ft_balance(token.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {tokens}");

    // Transfer 100 tokens to the account
    // We handle internally the storage deposit for the receiver account
    Tokens::account(token.clone())
        .send_to(account.clone())
        .ft(
            token.clone(),
            // Send 1.5 tokens
            FTBalance::with_decimals(24).with_whole_amount(100),
        )
        .unwrap()
        .with_signer(token_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let tokens = Tokens::account(account.clone())
        .ft_balance(token.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Account has {tokens}");

    let tokens = Tokens::account(token.clone())
        .ft_balance(token.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {tokens}");

    // We validate decimals at the network level so this should fail with a validation error
    let token = Tokens::account(token.clone())
        .send_to(account.clone())
        .ft(
            token.clone(),
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
