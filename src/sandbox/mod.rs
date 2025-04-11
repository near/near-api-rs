use std::net::SocketAddrV4;
use std::sync::Arc;
use std::{fs::File, net::Ipv4Addr};

use crate::errors::SandboxError;
use crate::signer::{generate_secret_key, AccountKeyPair};
use crate::{Account, NetworkConfig, Signer};
use fs2::FileExt;
use near_account_id::AccountId;
use near_crypto::SecretKey;
use near_primitives::views::FinalExecutionStatus;
use near_token::NearToken;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::process::Child;
use tracing::info;
use url::Url;

pub mod config;

// Must be an IP address as `neard` expects socket address for network address.
const DEFAULT_RPC_HOST: &str = "127.0.0.1";

const INITIAL_BALANCE: NearToken = NearToken::from_near(10);

fn rpc_socket(port: u16) -> String {
    format!("{DEFAULT_RPC_HOST}:{}", port)
}

/// Request an unused port from the OS.
async fn pick_unused_port() -> Result<u16, SandboxError> {
    // Port 0 means the OS gives us an unused port
    // Important to use localhost as using 0.0.0.0 leads to users getting brief firewall popups to
    // allow inbound connections on MacOS.
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| SandboxError::Io(err))?;
    let port = listener
        .local_addr()
        .map_err(|err| SandboxError::Io(err))?
        .port();
    Ok(port)
}

/// Acquire an unused port and lock it for the duration until the sandbox server has
/// been started.
async fn acquire_unused_port() -> Result<(u16, File), SandboxError> {
    loop {
        let port = pick_unused_port().await?;
        let lockpath = std::env::temp_dir().join(format!("near-sandbox-port{}.lock", port));
        let lockfile = File::create(lockpath)?;
        if lockfile.try_lock_exclusive().is_ok() {
            break Ok((port, lockfile));
        }
    }
}

/// An sandbox instance that can be used to launch local near network to test against.
///
/// All the [examples](https://github.com/near/near-api-rs/tree/main/examples) are using Sandbox implementation.
///
/// This is work-in-progress and not all the features are supported yet.
pub struct Sandbox {
    pub validator_account_id: AccountId,
    pub validator_key: SecretKey,
    pub validator_signer: Arc<Signer>,
    pub home_dir: TempDir,
    pub network_config: NetworkConfig,
    pub rpc_port_lock: File,
    pub net_port_lock: File,
    process: Child,
}

impl Sandbox {
    /// Start a new sandbox with the default near-sandbox-utils version.
    pub async fn start_sandbox() -> Result<Self, SandboxError> {
        Self::start_sandbox_with_version(near_sandbox_utils::DEFAULT_NEAR_SANDBOX_VERSION).await
    }

    /// Create a new root subaccount with the initial balance.
    ///
    /// # Arguments
    /// * `account_id` - the account id of the new subaccount. Should be a sub-account of the sandbox account.
    pub async fn create_root_subaccount(
        &self,
        account_id: &AccountId,
    ) -> Result<SecretKey, SandboxError> {
        self.create_root_subaccount_with_balance(account_id, INITIAL_BALANCE)
            .await
    }

    /// Create a new root subaccount with the initial balance.
    ///
    /// # Arguments
    /// * `account_id` - the account id of the new subaccount. Should be a sub-account of the sandbox account.
    /// * `initial_balance` - the initial balance of the new subaccount.
    ///
    pub async fn create_root_subaccount_with_balance(
        &self,
        account_id: &AccountId,
        initial_balance: NearToken,
    ) -> Result<SecretKey, SandboxError> {
        if !account_id.is_sub_account_of(&self.validator_account_id) {
            return Err(SandboxError::InvalidAccountId);
        }

        let secret_key = generate_secret_key()?;

        let account = Account::create_account(account_id.clone())
            .fund_myself(self.validator_account_id.clone(), initial_balance)
            .public_key(secret_key.public_key())?
            .with_signer(self.validator_signer.clone())
            .send_to(&self.network_config)
            .await?;

        if let FinalExecutionStatus::Failure(e) = account.status {
            return Err(SandboxError::TransactionFailed(e));
        }

        Ok(secret_key)
    }

    /// Start a new sandbox with the given near-sandbox-utils version.
    ///
    /// # Arguments
    /// * `version` - the version of the near-sandbox-utils to use.
    ///
    pub async fn start_sandbox_with_version(version: &str) -> Result<Self, SandboxError> {
        suppress_sandbox_logs_if_required();
        let home_dir = Self::init_home_dir_with_version(version).await?;

        let (rpc_port, rpc_port_lock) = acquire_unused_port().await?;
        let (net_port, net_port_lock) = acquire_unused_port().await?;

        let rpc_addr = rpc_socket(rpc_port);
        let net_addr = rpc_socket(net_port);

        config::set_sandbox_configs(&home_dir)?;
        config::set_sandbox_genesis(&home_dir)?;

        let options = &[
            "--home",
            home_dir.path().to_str().expect("home_dir is valid utf8"),
            "run",
            "--rpc-addr",
            &rpc_addr,
            "--network-addr",
            &net_addr,
        ];

        let child = near_sandbox_utils::run_with_options_with_version(options, version)
            .map_err(|e| SandboxError::RunFailure(e.to_string()))?;

        info!(target: "sandbox", "Started up sandbox at localhost:{} with pid={:?}", rpc_port, child.id());

        let rpc_addr: Url = format!("http://{rpc_addr}")
            .parse()
            .expect("static scheme and host name with variable u16 port numbers form valid urls");

        let validator_file = home_dir.path().join("registrar.json");
        let validator_key = AccountKeyPair::load_access_key_file(&validator_file)?;

        Ok(Self {
            validator_account_id: validator_key.account_id,
            validator_key: validator_key.private_key.clone(),
            validator_signer: Signer::new(Signer::from_secret_key(validator_key.private_key))?,
            home_dir,
            network_config: NetworkConfig::sandbox(rpc_addr),
            rpc_port_lock,
            net_port_lock,
            process: child,
        })
    }

    async fn init_home_dir_with_version(version: &str) -> Result<TempDir, SandboxError> {
        let home_dir = tempfile::tempdir()?;

        let output = near_sandbox_utils::init_with_version(&home_dir, version)
            .map_err(|e| SandboxError::InitFailure(e.to_string()))?
            .wait_with_output()
            .await
            .map_err(|e| SandboxError::InitFailure(e.to_string()))?;

        info!(target: "sandbox", "sandbox init: {:?}", output);

        Ok(home_dir)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        info!(
            target: "sandbox",
            "Cleaning up sandbox: pid={:?}",
            self.process.id()
        );

        self.process.start_kill().expect("failed to kill sandbox");
        let _ = self.process.try_wait();
    }
}

/// Turn off neard-sandbox logs by default. Users can turn them back on with
/// NEAR_ENABLE_SANDBOX_LOG=1 and specify further parameters with the custom
/// NEAR_SANDBOX_LOG for higher levels of specificity. NEAR_SANDBOX_LOG args
/// will be forward into RUST_LOG environment variable as to not conflict
/// with similar named log targets.
fn suppress_sandbox_logs_if_required() {
    if let Ok(val) = std::env::var("NEAR_ENABLE_SANDBOX_LOG") {
        if val != "0" {
            return;
        }
    }

    // non-exhaustive list of targets to suppress, since choosing a default LogLevel
    // does nothing in this case, since nearcore seems to be overriding it somehow:
    std::env::set_var("NEAR_SANDBOX_LOG", "near=error,stats=error,network=error");
}
