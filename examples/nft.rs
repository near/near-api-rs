use near_api::prelude::*;

use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_token::NearToken;
use serde_json::json;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let nft = network.dev_create_account().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let contract = Contract(nft.id().clone());
    let nft_signer = Signer::new(Signer::from_workspace(&nft)).unwrap();

    // Deploying token contract
    contract
        .deploy(include_bytes!("../resources/nft.wasm").to_vec())
        .with_init_call(
            "new_default_meta",
            json!({
                "owner_id": nft.id().to_string(),
            }),
        )
        .unwrap()
        .with_signer(nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    // Mint NFT via contract call
    contract
        .call_function(
            "nft_mint",
            json!({
                "token_id": "1",
                "receiver_id": account.id().to_string(),
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
        .with_signer(nft.id().clone(), nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    // Verifying that account has our nft token
    let tokens = Tokens::of(account.id().clone())
        .nft_assets(nft.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("Account has {}", tokens.data.first().unwrap().token_id);

    Tokens::of(account.id().clone())
        .send_to(nft.id().clone())
        .nft(nft.id().clone(), "1".to_string())
        .unwrap()
        .with_signer(nft_signer.clone())
        .send_to(&network)
        .await
        .unwrap();

    // Verifying that account doesn't have nft anymore
    let tokens = Tokens::of(account.id().clone())
        .nft_assets(nft.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert!(tokens.data.is_empty());

    let tokens = Tokens::of(nft.id().clone())
        .nft_assets(nft.id().clone())
        .unwrap()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(tokens.data.len(), 1);
    println!("nft has {}", tokens.data.first().unwrap().token_id);
}
