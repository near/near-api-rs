use near_api::{
    types::{AccountId, NearToken, nft::TokenMetadata},
    *,
};
use near_sandbox_utils::{GenesisAccount, SandboxConfig};
use serde_json::json;

#[tokio::main]
async fn main() {
    let nft: AccountId = "nft.testnet".parse().unwrap();
    let account: AccountId = "account.testnet".parse().unwrap();
    let account2: AccountId = "account2.testnet".parse().unwrap();

    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: nft.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network = NetworkConfig::from_sandbox(&network);

    let nft_signer = Signer::new(Signer::default_sandbox()).unwrap();
    let account_signer = Signer::new(Signer::default_sandbox()).unwrap();

    // Deploying token contract
    Contract::deploy(nft.clone())
        .use_code(include_bytes!("../resources/nft.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": nft.to_string(),
            }),
        )
        .unwrap()
        .with_signer(nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let contract = Contract(nft.clone());

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
        .with_signer(nft.clone(), nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verifying that account has our nft token
    let tokens = Tokens::account(account.clone())
        .nft_assets(nft.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("Account has {}", tokens.data.first().unwrap().token_id);

    Tokens::account(account.clone())
        .send_to(account2.clone())
        .nft(nft.clone(), "1".to_string())
        .unwrap()
        .with_signer(account_signer.clone())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    // Verifying that account doesn't have nft anymore
    let tokens = Tokens::account(account.clone())
        .nft_assets(nft.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert!(tokens.data.is_empty());

    let tokens = Tokens::account(account2.clone())
        .nft_assets(nft.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("account 2 has {}", tokens.data.first().unwrap().token_id);
}
