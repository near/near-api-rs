use near_api::*;
use near_primitives::{account::AccessKeyPermission, views::AccessKeyPermissionView};
use signer::generate_secret_key;

#[tokio::test]
async fn create_and_delete_account() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let account_id = "account.sandbox".parse().unwrap();
    let account_sk = sandbox.create_root_subaccount(&account_id).await.unwrap();
    let network = &sandbox.network_config;

    let signer = Signer::new(Signer::from_secret_key(account_sk)).unwrap();

    let new_account: AccountId = format!("{}.{}", "bob", account_id).parse().unwrap();
    let secret = generate_secret_key().unwrap();
    Account::create_account(new_account.clone())
        .fund_myself(account_id.clone(), NearToken::from_near(1))
        .public_key(secret.public_key())
        .unwrap()
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let balance_before_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(network)
        .await
        .unwrap();

    assert_eq!(balance_before_del.total.as_near(), 1);

    Account(account_id.clone())
        .delete_account_with_beneficiary(new_account.clone())
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    Tokens::account(account_id.clone())
        .near_balance()
        .fetch_from(network)
        .await
        .expect_err("Shouldn't exist");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let balance_after_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(network)
        .await
        .unwrap();
    assert!(balance_after_del.total > balance_before_del.total);
}

#[tokio::test]
async fn transfer_funds() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let alice = "alice.sandbox".parse().unwrap();
    let bob = "bob.sandbox".parse().unwrap();
    let alice_sk = sandbox
        .create_root_subaccount_with_balance(&alice, NearToken::from_near(100))
        .await
        .unwrap();
    let _bob_sk = sandbox.create_root_subaccount(&bob).await.unwrap();
    let network = &sandbox.network_config;

    let signer = Signer::new(Signer::from_secret_key(alice_sk)).unwrap();

    Tokens::account(alice.clone())
        .send_to(bob.clone())
        .near(NearToken::from_near(50))
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let alice_balance = Tokens::account(alice.clone())
        .near_balance()
        .fetch_from(network)
        .await
        .unwrap();

    let bob_balance = Tokens::account(bob.clone())
        .near_balance()
        .fetch_from(network)
        .await
        .unwrap();

    // it's actually 49.99 because of the fee
    assert_eq!(alice_balance.total.as_near(), 49);
    assert_eq!(bob_balance.total.as_near(), 60);
}

#[tokio::test]
async fn access_key_management() {
    let sandbox = near_api::sandbox::Sandbox::start_sandbox().await.unwrap();
    let alice = "alice.sandbox".parse().unwrap();
    let alice_sk = sandbox.create_root_subaccount(&alice).await.unwrap();
    let network = &sandbox.network_config;

    let signer = Signer::new(Signer::from_secret_key(alice_sk)).unwrap();

    let alice_acc = Account(alice.clone());

    let keys = alice_acc.list_keys().fetch_from(network).await.unwrap();
    assert_eq!(keys.keys.len(), 1);

    let secret = generate_secret_key().unwrap();

    alice_acc
        .add_key(AccessKeyPermission::FullAccess, secret.public_key())
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(network).await.unwrap();
    assert_eq!(keys.keys.len(), 2);

    let new_key_info = alice_acc
        .access_key(secret.public_key())
        .fetch_from(network)
        .await
        .unwrap();

    assert_eq!(
        new_key_info.data.permission,
        AccessKeyPermissionView::FullAccess
    );

    alice_acc
        .delete_key(secret.public_key())
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(network).await.unwrap();

    assert_eq!(keys.keys.len(), 1);

    alice_acc
        .access_key(secret.public_key())
        .fetch_from(network)
        .await
        .expect_err("Shouldn't exist");

    for _ in 0..10 {
        let secret = generate_secret_key().unwrap();
        alice_acc
            .add_key(AccessKeyPermission::FullAccess, secret.public_key())
            .with_signer(signer.clone())
            .send_to(network)
            .await
            .unwrap()
            .assert_success();
    }

    let keys = alice_acc.list_keys().fetch_from(network).await.unwrap();

    assert_eq!(keys.keys.len(), 11);

    alice_acc
        .delete_keys(keys.keys.into_iter().map(|k| k.public_key).collect())
        .with_signer(signer.clone())
        .send_to(network)
        .await
        .unwrap()
        .assert_success();

    let keys = alice_acc.list_keys().fetch_from(network).await.unwrap();
    assert_eq!(keys.keys.len(), 0);
}
