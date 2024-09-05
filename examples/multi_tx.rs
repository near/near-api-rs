use near::{signer::Signer, Account, MultiTransactions, NetworkConfig, Tokens};
use near_primitives::account::AccessKeyPermission;
use near_sdk::NearToken;

#[tokio::main]
async fn main() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();

    let tmp_account = network.dev_create_account().await.unwrap();
    let network = NetworkConfig::from(network);

    // Let's add new key and get the seed phrase
    let (new_seed_phrase, tx) = Account(account.id().clone())
        .add_key(AccessKeyPermission::FullAccess)
        .new_keypair()
        .generate_seed_phrase()
        .unwrap();
    tx.with_signer(Signer::from_workspace(&account))
        .send_to(&network)
        .await
        .unwrap()
        .first()
        .assert_success();

    let multi_tx = (0..8)
        .map(|i| {
            Tokens::of(account.id().clone())
                .send_to(tmp_account.id().clone())
                .near(NearToken::from_near(i + 1))
        })
        .fold(MultiTransactions::new(), |multi_tx, tx| {
            multi_tx.add_transaction(tx)
        })
        // We have only 2 signers, but we want to send 8 transactions.
        // Each signer key will be used for 4 transactions.
        // This optional parameter allows us to send transactions with the same signer concurrently.
        // It's more efficient, but might fail if transaction with higher nonce arrives first
        .with_same_signer_concurrent(true)
        .with_signers(vec![
            Signer::seed_phrase(new_seed_phrase.clone(), None).unwrap(),
            Signer::from_workspace(&account),
        ])
        .unwrap();

    let result = multi_tx.send_to(&network).await.unwrap();

    assert_eq!(result.len(), 8);
    result.iter().for_each(|r| r.assert_success());

    println!(
        "TMP Account balance: {}",
        Tokens::of(tmp_account.id().clone())
            .near_balance()
            .fetch_from(&network)
            .await
            .unwrap()
            .liquid
    );
}
