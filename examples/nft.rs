use near_api::*;

use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use serde_json::json;

#[tokio::main]
async fn main() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let nft_id = "nft.sandbox".parse().unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let account2_id = "account2.sandbox".parse().unwrap();
    let nft_sk = sandbox.create_root_subaccount(&nft_id).await.unwrap();
    let _account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let _account2_sk = sandbox.create_root_subaccount(&account2_id).await.unwrap();
    let network = &sandbox.network_config;

    let nft_signer = Signer::new(Signer::from_secret_key(nft_sk)).unwrap();

    // Deploying token contract
    Contract::deploy(
        nft_id.clone(),
        include_bytes!("../resources/nft.wasm").to_vec(),
    )
    .with_init_call(
        "new_default_meta",
        json!({
            "owner_id": nft_id.to_string(),
        }),
    )
    .unwrap()
    .with_signer(nft_signer.clone())
    .send_to(&network)
    .await
    .unwrap();

    let contract = Contract(nft_id.clone());

    // Mint NFT via contract call
    contract
        .call_function(
            "nft_mint",
            json!({
                "token_id": "1",
                "receiver_id": account_id.to_string(),
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
        .with_signer(nft_id.clone(), nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    // Verifying that account has our nft token
    let tokens = Tokens::account(account_id.clone())
        .nft_assets(nft_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("Account has {}", tokens.data.first().unwrap().token_id);

    Tokens::account(account_id.clone())
        .send_to(account2_id.clone())
        .nft(nft_id.clone(), "1".to_string())
        .unwrap()
        .with_signer(nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    // Verifying that account doesn't have nft anymore
    let tokens = Tokens::account(account_id.clone())
        .nft_assets(nft_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert!(tokens.data.is_empty());

    let tokens = Tokens::account(account2_id.clone())
        .nft_assets(nft_id.clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("account 2 has {}", tokens.data.first().unwrap().token_id);
}
