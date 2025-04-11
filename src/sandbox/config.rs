//! Network specific configurations used to modify behavior inside a chain.
//! This is so far only useable with sandbox networks since it would require
//! direct access to a node to change the config. Each network like mainnet
//! and testnet already have pre-configured settings; meanwhile sandbox can
//! have additional settings on top of them to facilitate custom behavior
//! such as sending large requests to the sandbox network.
//
// NOTE: nearcore has many, many configs which can easily change in the future
// so this config.rs file just purely modifies the data and does not try to
// replicate all the structs from nearcore side; which can be a huge maintenance
// churn if we were to.

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::str::FromStr;

use serde_json::Value;

use crate::errors::SandboxError;

/// Overwrite the $home_dir/config.json file over a set of entries. `value` will be used per (key, value) pair
/// where value can also be another dict. This recursively sets all entry in `value` dict to the config
/// dict, and saves back into `home_dir` at the end of the day.
fn overwrite(home_dir: impl AsRef<Path>, value: Value) -> Result<(), SandboxError> {
    let home_dir = home_dir.as_ref();
    let config_file = File::open(home_dir.join("config.json")).map_err(SandboxError::IO)?;
    let config = BufReader::new(config_file);
    let mut config: Value = serde_json::from_reader(config)
        .map_err(|err| SandboxError::ConfigError(err.to_string()))?;

    json_patch::merge(&mut config, &value);
    let config_file = File::create(home_dir.join("config.json")).map_err(SandboxError::IO)?;
    serde_json::to_writer(config_file, &config)
        .map_err(|err| SandboxError::ConfigError(err.to_string()))?;

    Ok(())
}

/// Parse an environment variable or return a default value.
fn parse_env<T>(env_var: &str) -> Result<Option<T>, SandboxError>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match std::env::var(env_var) {
        Ok(val) => {
            let val = val
                .parse::<T>()
                .map_err(|err| SandboxError::ConfigError(err.to_string()))?;
            Ok(Some(val))
        }
        Err(_err) => Ok(None),
    }
}

/// Set extra configs for the sandbox defined by workspaces.
pub(crate) fn set_sandbox_configs(home_dir: impl AsRef<Path>) -> Result<(), SandboxError> {
    overwrite(
        home_dir,
        serde_json::json!({
            "rpc": {
                "limits_config": {
                    // default to 1GB payload size so that large state patches can work.
                    "json_payload_max_size": parse_env("NEAR_SANDBOX_MAX_PAYLOAD_SIZE")?.unwrap_or(1024 * 1024 * 1024),
                },
            },
            "store": {
                // default to 3,000 files open at a time so that windows WSL can work without configuring.
                "max_open_files": parse_env("NEAR_SANDBOX_MAX_FILES")?.unwrap_or(3000),
            }
        }),
    )
}

/// Overwrite the $home_dir/genesis.json file over a set of entries. `value` will be used per (key, value) pair
/// where value can also be another dict. This recursively sets all entry in `value` dict to the config
/// dict, and saves back into `home_dir` at the end of the day.
fn overwrite_genesis(home_dir: impl AsRef<Path>) -> Result<(), SandboxError> {
    let home_dir = home_dir.as_ref();
    let config_file = File::open(home_dir.join("genesis.json")).map_err(SandboxError::IO)?;
    let config = BufReader::new(config_file);
    let mut config: Value = serde_json::from_reader(config)
        .map_err(|err| SandboxError::ConfigError(err.to_string()))?;

    let config = config.as_object_mut().expect("expected to be object");
    let mut total_supply = u128::from_str(
        config
            .get_mut("total_supply")
            .expect("expected exist total_supply")
            .as_str()
            .unwrap_or_default(),
    )
    .unwrap_or_default();
    let registrar_amount = 10_000_000_000_000_000_000_000_000_000_u128;
    total_supply += registrar_amount;
    config.insert(
        "total_supply".to_string(),
        Value::String(total_supply.to_string()),
    );
    let records = config.get_mut("records").expect("expect exist records");
    records
        .as_array_mut()
        .expect("expected to be array")
        .push(serde_json::json!(
            {
                  "Account": {
                    "account_id": "sandbox",
                    "account": {
                      "amount": registrar_amount.to_string(),
                      "locked": "0",
                      "code_hash": "11111111111111111111111111111111",
                      "storage_usage": 182
                    }
                  }
            }
        ));
    records
        .as_array_mut()
        .expect("expected to be array")
        .push(serde_json::json!(
            {
              "AccessKey": {
                "account_id": "sandbox",
                "public_key": "ed25519:5BGSaf6YjVm7565VzWQHNxoyEjwr3jUpRJSGjREvU9dB",
                "access_key": {
                  "nonce": 0,
                  "permission": "FullAccess"
                }
              }
            }
        ));

    let config_file = File::create(home_dir.join("genesis.json")).map_err(SandboxError::IO)?;
    serde_json::to_writer(config_file, &config)
        .map_err(|err| SandboxError::ConfigError(err.to_string()))?;

    Ok(())
}

pub fn set_sandbox_genesis(home_dir: impl AsRef<Path>) -> Result<(), SandboxError> {
    overwrite_genesis(&home_dir)?;
    let registrar_key = r#"{"account_id":"sandbox","public_key":"ed25519:5BGSaf6YjVm7565VzWQHNxoyEjwr3jUpRJSGjREvU9dB","private_key":"ed25519:3tgdk2wPraJzT4nsTuf86UX41xgPNk3MHnq8epARMdBNs29AFEztAuaQ7iHddDfXG9F2RzV1XNQYgJyAyoW51UBB"}"#;
    let mut registrar_wallet =
        File::create(home_dir.as_ref().join("registrar.json")).map_err(SandboxError::IO)?;
    registrar_wallet
        .write_all(registrar_key.as_bytes())
        .map_err(SandboxError::IO)?;
    registrar_wallet.flush().map_err(SandboxError::IO)?;
    Ok(())
}
