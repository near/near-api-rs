use near_api::{
    types::{AccountId, NearToken, nft::TokenMetadata},
    *,
};
use near_sandbox::{GenesisAccount, SandboxConfig, config::DEFAULT_GENESIS_ACCOUNT};
use serde_json::json;

#[tokio::main]
async fn main() {
    let nft = GenesisAccount::generate_with_name("nft".parse().unwrap());
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();
    let account2 = GenesisAccount::generate_with_name("account2".parse().unwrap());

    let network = near_sandbox::Sandbox::start_sandbox_with_config(SandboxConfig {
        additional_accounts: vec![nft.clone(), account2.clone()],
        ..Default::default()
    })
    .await
    .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    let nft_signer = Signer::new(Signer::from_secret_key(
        nft.private_key.clone().parse().unwrap(),
    ))
    .unwrap();
    let account_signer = Signer::new(Signer::default_sandbox()).unwrap();

    // Deploying token contract
    Contract::deploy(nft.account_id.clone())
        .use_code(include_bytes!("../resources/nft.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": nft.account_id.to_string(),
            }),
        )
        .unwrap()
        .with_signer(nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(nft.account_id.clone());

    // Mint NFT via contract call
    contract
        .call_function(
            "nft_mint",
            json!({
                "token_id": "1",
                "receiver_id": account.to_string(),
                "token_metadata": TokenMetadata {
                    title: Some("My NFT".to_string()),
                    description: Some("My first NFT".to_string()),
                    ..Default::default()
                }
            }),
        )
        .unwrap()
        .transaction()
        .deposit(NearToken::from_millinear(100))
        .with_signer(nft.account_id.clone(), nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verifying that account has our nft token
    let tokens = Tokens::account(account.clone())
        .nft_assets(nft.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("Account has {}", tokens.data.first().unwrap().token_id);

    Tokens::account(account.clone())
        .send_to(account2.account_id.clone())
        .nft(nft.account_id.clone(), "1".to_string())
        .unwrap()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verifying that account doesn't have nft anymore
    let tokens = Tokens::account(account.clone())
        .nft_assets(nft.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert!(tokens.data.is_empty());

    let tokens = Tokens::account(account2.account_id.clone())
        .nft_assets(nft.account_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("account 2 has {}", tokens.data.first().unwrap().token_id);
}
