use std::str::FromStr;
use near_api::*;

#[tokio::main]
async fn main() {
    for (account_name, expected_json_metadata) in [
        ("desolate-toad.testnet", FIRST_METADATA),
        ("fat-fabulous-toad.testnet", SECOND_METADATA),
    ] {
        let source_metadata = Contract(AccountId::from_str(account_name).expect("no err"))
            .contract_source_metadata()
            .fetch_from_testnet()
            .await
            .expect("no network or rpc err");

        assert_eq!(
            expected_json_metadata,
            serde_json::to_string_pretty(&source_metadata.data).expect("no ser err")
        );

    }
}

const FIRST_METADATA: &str = r#"{
  "version": "0.1.0",
  "link": "https://github.com/dj8yfo/quiet_glen",
  "standards": [
    {
      "standard": "nep330",
      "version": "1.2.0"
    }
  ],
  "build_info": null
}"#;

const SECOND_METADATA: &str = r#"{
  "version": "0.1.0",
  "link": "https://github.com/dj8yfo/quiet_glen/tree/8d8a8a0fe86a1d8eb3bce45f04ab1a65fecf5a1b",
  "standards": [
    {
      "standard": "nep330",
      "version": "1.2.0"
    }
  ],
  "build_info": {
    "build_environment": "sourcescan/cargo-near:0.13.3-rust-1.84.0@sha256:722198ddb92d1b82cbfcd3a4a9f7fba6fd8715f4d0b5fb236d8725c4883f97de",
    "build_command": [
      "cargo",
      "near",
      "build",
      "non-reproducible-wasm",
      "--locked"
    ],
    "contract_path": "",
    "source_code_snapshot": "git+https://github.com/dj8yfo/quiet_glen?rev=8d8a8a0fe86a1d8eb3bce45f04ab1a65fecf5a1b"
  }
}"#;
