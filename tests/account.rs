use near_account_id::AccountId;
use near_api::prelude::*;
use near_api::types::views::AccessKeyPermission;
use near_token::NearToken;

#[tokio::test]
async fn create_and_delete_account() {
    let network = near_workspaces::sandbox().await.unwrap();
    let account = network.dev_create_account().await.unwrap();
    let network: NetworkConfig = NetworkConfig::from(network);

    let new_account: AccountId = format!("{}.{}", "bob", account.id()).parse().unwrap();

    Account::create_account()
        .fund_myself(
            new_account.clone(),
            account.id().clone(),
            NearToken::from_near(1),
        )
        .new_keypair()
        .generate_secret_key()
        .unwrap()
        .1
        .with_signer(Signer::new(Signer::from_workspace(&account)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let balance_before_del = Tokens::of(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(balance_before_del.liquid.as_near(), 1);

    dbg!(Account(account.id().clone())
        .delete_account_with_beneficiary(new_account.clone())
        .with_signer(Signer::new(Signer::from_workspace(&account)).unwrap())
        .send_to(&network)
        .await
        .unwrap())
    .assert_success();

    Tokens::of(account.id().clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    // TODO: why do we need a sleep to wait for beneficiary transfer?
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let balance_after_del = Tokens::of(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();
    assert!(dbg!(balance_after_del).liquid > dbg!(balance_before_del).liquid);
}

#[tokio::test]
async fn transfer_funds() {
    let network = near_workspaces::sandbox().await.unwrap();
    let alice = network.dev_create_account().await.unwrap();
    let bob = network.dev_create_account().await.unwrap();
    let network: NetworkConfig = NetworkConfig::from(network);

    Tokens::of(alice.id().clone())
        .send_to(bob.id().clone())
        .near(NearToken::from_near(50))
        .with_signer(Signer::new(Signer::from_workspace(&alice)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let alice_balance = Tokens::of(alice.id().clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    let bob_balance = Tokens::of(bob.id().clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    // it's acctually 49.99 because of the fee
    assert_eq!(alice_balance.liquid.as_near(), 49);
    assert_eq!(bob_balance.liquid.as_near(), 150);
}

#[tokio::test]
async fn access_key_management() {
    let network = near_workspaces::sandbox().await.unwrap();
    let alice = network.dev_create_account().await.unwrap();
    let network: NetworkConfig = NetworkConfig::from(network);

    let alice_acc = Account(alice.id().clone());

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.keys.len(), 1);

    let (secret, tx) = alice_acc
        .add_key(AccessKeyPermission::FullAccess)
        .new_keypair()
        .generate_secret_key()
        .unwrap();

    tx.with_signer(Signer::new(Signer::from_workspace(&alice)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.keys.len(), 2);

    let new_key_info = alice_acc
        .access_key(secret.public_key())
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(
        new_key_info.data.permission,
        AccessKeyPermission::FullAccess
    );

    alice_acc
        .delete_key(secret.public_key())
        .with_signer(Signer::new(Signer::from_workspace(&alice)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();

    assert_eq!(keys.data.keys.len(), 1);

    alice_acc
        .access_key(secret.public_key())
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    for _ in 0..10 {
        alice_acc
            .add_key(AccessKeyPermission::FullAccess)
            .new_keypair()
            .generate_secret_key()
            .unwrap()
            .1
            .with_signer(Signer::new(Signer::from_workspace(&alice)).unwrap())
            .send_to(&network)
            .await
            .unwrap()
            .assert_success();
    }

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();

    assert_eq!(keys.data.keys.len(), 11);

    alice_acc
        .delete_keys(keys.data.keys.into_iter().map(|k| k.public_key).collect())
        .with_signer(Signer::new(Signer::from_workspace(&alice)).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.keys.len(), 0);
}
