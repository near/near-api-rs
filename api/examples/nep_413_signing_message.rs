use near_api::{NetworkConfig, Signer};

use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use openssl::rand::rand_bytes;

#[tokio::main]
async fn main() -> testresult::TestResult {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);

    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;
    let public_key = signer.get_public_key().await?;

    let mut nonce = [0u8; 32];
    rand_bytes(&mut nonce)?;

    let payload = near_api::signer::NEP413Payload {
        message: "Hello NEAR!".to_string(),
        nonce,
        recipient: "example.near".to_string(),
        callback_url: None,
    };

    let signature = signer
        .sign_message_nep413(DEFAULT_GENESIS_ACCOUNT.into(), public_key, &payload)
        .await?;

    println!("Signature: {signature}");

    let result = payload
        .verify(
            &DEFAULT_GENESIS_ACCOUNT.into(),
            public_key,
            &signature,
            &network,
        )
        .await?;
    assert!(result);

    Ok(())
}
