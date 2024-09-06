use std::sync::Arc;

use futures::future::join_all;
use near::{
    signer::{Signer, SignerTrait},
    Account, NetworkConfig, Tokens,
};
use near_sdk::NearToken;

#[tokio::test]
async fn multiple_tx_at_same_time_from_same_user() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();

    let tmp_account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    let start_nonce = Account(account.id().clone())
        .access_key(Signer::from_workspace(&account).get_public_key().unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;

    let tx = (0..100).map(|i| {
        Tokens::of(account.id().clone())
            .send_to(tmp_account.id().clone())
            .near(NearToken::from_millinear(i))
    });
    let signer = Signer::from_workspace(&account);
    let txs = join_all(tx.map(|t| t.with_signer(Arc::clone(&signer)).send_to(&network)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(txs.len(), 100);
    txs.iter().for_each(|a| a.assert_success());

    let end_nonce = Account(account.id().clone())
        .access_key(Signer::from_workspace(&account).get_public_key().unwrap())
        .fetch_from(&network)
        .await
        .unwrap()
        .data
        .nonce;
    assert_eq!(end_nonce, start_nonce + 100);
}
