use near_api::*;

use openssl::rand::rand_bytes;

#[tokio::main]
async fn main() {
    let signer = Signer::from_seed_phrase(
        "fatal edge jacket cash hard pass gallery fabric whisper size rain biology",
        None,
    )
    .unwrap();

    let mut nonce = [0u8; 32];
    rand_bytes(&mut nonce).unwrap();

    let payload = near_api::signer::NEP413Payload {
        message: "Hello NEAR!".to_string(),
        nonce,
        recipient: "example.near".to_string(),
        callback_url: None,
    };

    let signature = signer
        .sign_message_nep413(
            "round-toad.testnet".parse().unwrap(),
            signer.get_public_key().unwrap(),
            payload,
        )
        .await
        .unwrap();

    println!("Signature: {}", signature);
}
