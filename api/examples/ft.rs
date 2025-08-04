use near_api::{
    types::{AccountId, tokens::FTBalance},
    *,
};
use near_sandbox::{GenesisAccount, SandboxConfig, config::DEFAULT_GENESIS_ACCOUNT};

use serde_json::json;

#[tokio::main]
async fn main() {
    let token = GenesisAccount::generate_with_name("token".parse().unwrap());
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let token_signer = Signer::new(Signer::from_secret_key(
        token.private_key.clone().parse().unwrap(),
    ))
    .unwrap();

    let sandbox = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![token.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_sandbox(&sandbox);

    // Deploying token contract
    Contract::deploy(token.account_id.clone())
        .use_code(include_bytes!("../resources/fungible_token.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                    "owner_id": token.account_id.clone(),
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
    let tokens = Tokens::account(token.account_id.clone())
        .ft_balance(token.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {tokens}");

    // Transfer 100 tokens to the account
    // We handle internally the storage deposit for the receiver account
    Tokens::account(token.account_id.clone())
        .send_to(account.clone())
        .ft(
            token.account_id.clone(),
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
        .ft_balance(token.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Account has {tokens}");

    let tokens = Tokens::account(token.account_id.clone())
        .ft_balance(token.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    println!("Owner has {tokens}");

    // We validate decimals at the network level so this should fail with a validation error
    let token = Tokens::account(token.account_id.clone())
        .send_to(account.clone())
        .ft(
            token.account_id.clone(),
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
