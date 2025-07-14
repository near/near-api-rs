use near_api::{
    types::{AccessKeyPermission, AccountId, NearToken, TxExecutionStatus},
    *,
};
use near_sandbox_utils::{
    GenesisAccount, SandboxConfig, high_level::config::DEFAULT_GENESIS_ACCOUNT,
};
use signer::generate_secret_key;

#[tokio::test]
async fn create_and_delete_account() {
    let network = near_sandbox_utils::high_level::Sandbox::start_sandbox()
        .await
        .unwrap();

    let account_id: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let network: NetworkConfig = NetworkConfig::from_sandbox(&network);

    let new_account: AccountId = format!("{}.{}", "bob", account_id).parse().unwrap();
    let secret = generate_secret_key().unwrap();
    let public_key = secret.public_key();

    Account::create_account(new_account.clone())
        .fund_myself(account_id.clone(), NearToken::from_near(1))
        .public_key(public_key)
        .unwrap()
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let balance_before_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(balance_before_del.total.as_near(), 1);

    Account(account_id.clone())
        .delete_account_with_beneficiary(new_account.clone())
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .wait_until(TxExecutionStatus::Final)
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    Tokens::account(account_id.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    let balance_after_del = Tokens::account(new_account.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();
    assert!(balance_after_del.total > balance_before_del.total);
}

#[tokio::test]
async fn transfer_funds() {
    let alice: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();
    let bob: AccountId = format!("{}.{}", "bob", DEFAULT_GENESIS_ACCOUNT)
        .parse()
        .unwrap();
    let network =
        near_sandbox_utils::high_level::Sandbox::start_sandbox_with_config(SandboxConfig {
            additional_accounts: vec![GenesisAccount {
                account_id: bob.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    let network: NetworkConfig = NetworkConfig::from_sandbox(&network);

    Tokens::account(alice.clone())
        .send_to(bob.clone())
        .near(NearToken::from_near(50))
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let alice_balance = Tokens::account(alice.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    let bob_balance = Tokens::account(bob.clone())
        .near_balance()
        .fetch_from(&network)
        .await
        .unwrap();

    // it's actually 49.99 because of the fee
    assert_eq!(alice_balance.total.as_near(), 9949);
    assert_eq!(bob_balance.total.as_near(), 10050);
}

#[tokio::test]
async fn access_key_management() {
    let network = near_sandbox_utils::high_level::Sandbox::start_sandbox()
        .await
        .unwrap();
    let network: NetworkConfig = NetworkConfig::from_sandbox(&network);
    let alice: AccountId = DEFAULT_GENESIS_ACCOUNT.parse().unwrap();

    let alice_acc = Account(alice.clone());

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.len(), 1);

    let secret = generate_secret_key().unwrap();
    let public_key = secret.public_key();

    alice_acc
        .add_key(AccessKeyPermission::FullAccess, public_key.clone())
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.len(), 2);

    let new_key_info = alice_acc
        .access_key(public_key.clone())
        .fetch_from(&network)
        .await
        .unwrap();

    assert_eq!(
        new_key_info.data.permission,
        AccessKeyPermission::FullAccess
    );

    alice_acc
        .delete_key(secret.public_key())
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();

    assert_eq!(keys.data.len(), 1);

    alice_acc
        .access_key(secret.public_key())
        .fetch_from(&network)
        .await
        .expect_err("Shouldn't exist");

    for _ in 0..10 {
        let secret = generate_secret_key().unwrap();
        alice_acc
            .add_key(AccessKeyPermission::FullAccess, secret.public_key())
            .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
            .send_to(&network)
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();

    assert_eq!(keys.data.len(), 11);

    alice_acc
        .delete_keys(keys.data.into_iter().map(|k| k.public_key).collect())
        .with_signer(Signer::new(Signer::default_sandbox()).unwrap())
        .send_to(&network)
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let keys = alice_acc.list_keys().fetch_from(&network).await.unwrap();
    assert_eq!(keys.data.len(), 0);
}
