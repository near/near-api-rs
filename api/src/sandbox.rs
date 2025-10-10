use std::sync::Arc;

use base64::{Engine, prelude::BASE64_STANDARD};
use near_api_types::{
    AccessKey, AccessKeyPermission, AccountId, CryptoHash, NearToken, PublicKey, Reference,
    account::ContractState, sandbox::StateRecord,
};
use near_openapi_client::types::StateItem;

use crate::{
    Account, Contract, NetworkConfig, Signer,
    advanced::{
        RequestBuilder, RpcBuilder,
        sandbox_rpc::{SandboxAction, SimpleSandboxRpc},
    },
    errors::SandboxError,
    signer::generate_secret_key,
};

#[derive(Clone, Debug, Copy)]
pub struct Sandbox;

impl Sandbox {
    pub fn import_account(source_account: AccountId) -> PatchAccount {
        PatchAccount::new(source_account)
    }

    pub const fn patch_transaction(
        account: AccountId,
        account_data: near_api_types::Account,
    ) -> PatchTransaction {
        PatchTransaction::new(account, account_data)
    }

    pub fn fast_forward(height: u64) -> RpcBuilder<SimpleSandboxRpc, SimpleSandboxRpc> {
        let rpc: SimpleSandboxRpc = SimpleSandboxRpc {
            action: SandboxAction::FastForward(height),
        };

        RequestBuilder::new(rpc.clone(), (), rpc)
    }
}

#[derive(Clone, Debug)]
pub struct PatchAccount {
    source_account: AccountId,

    new_balance: Option<NearToken>,
    block_ref: Reference,

    import_state: bool,

    destination_account: AccountId,

    network: NetworkConfig,
}

impl PatchAccount {
    pub fn new(source_account: AccountId) -> Self {
        Self {
            source_account: source_account.clone(),
            new_balance: None,
            block_ref: Reference::Optimistic,
            import_state: false,
            destination_account: source_account,
            network: NetworkConfig::mainnet(),
        }
    }

    /// Set the block reference to fetch the source account from.
    ///
    /// Defaults to [`Reference::Optimistic`].
    pub const fn at(mut self, block_ref: Reference) -> Self {
        self.block_ref = block_ref;
        self
    }

    /// Import the state of the source account into the sandbox.
    ///
    /// Defaults to `false`. Please note that large state accounts are not possible to fetch
    /// as RPC limits the size.
    pub const fn import_state(mut self) -> Self {
        self.import_state = true;
        self
    }

    /// Set the balance of the account.
    ///
    /// Defaults to the balance of the source account.
    pub const fn balance(mut self, balance: NearToken) -> Self {
        self.new_balance = Some(balance);
        self
    }

    /// Set the account to patch to.
    ///
    /// Defaults to the source account.
    pub fn dest_account(mut self, account_id: AccountId) -> Self {
        self.destination_account = account_id;
        self
    }

    /// Set the network to fetch the source account from.
    ///
    /// Defaults to [`NetworkConfig::mainnet()`].
    pub fn source_network(mut self, network: NetworkConfig) -> Self {
        self.network = network;
        self
    }

    /// Post the state patch to the sandbox network.
    ///
    /// This will return a signer that can be used to sign transactions for the destination account.
    ///
    /// Please note that the signer is not the same as the signer that was used to import the state.
    /// The signer is a new signer that is created for the destination account.
    pub async fn post_to(
        &self,
        destination_network: &NetworkConfig,
    ) -> Result<Arc<Signer>, SandboxError> {
        let mut account_view = Account(self.source_account.clone())
            .view()
            .at(self.block_ref.clone())
            .fetch_from(&self.network)
            .await?
            .data;

        if let Some(balance) = self.new_balance {
            account_view.amount = balance;
        }

        let secret_key = generate_secret_key()?;
        let code = account_view.contract_state != ContractState::None;
        let mut patch = PatchTransaction::new(self.destination_account.clone(), account_view)
            .access_key(
                secret_key.public_key(),
                AccessKey {
                    nonce: 0.into(),
                    permission: AccessKeyPermission::FullAccess,
                },
            );

        if code {
            let code = Contract(self.source_account.clone())
                .wasm()
                .at(self.block_ref.clone())
                .fetch_from(&self.network)
                .await?
                .data
                .code_base64;

            patch = patch.code(BASE64_STANDARD.decode(code)?.as_slice());
        }

        if self.import_state {
            let state = Contract(self.source_account.clone())
                .view_storage()
                .at(self.block_ref.clone())
                .fetch_from(&self.network)
                .await?
                .data;

            patch = patch.states(state.values);
        }

        patch.post(destination_network).await?;

        Ok(Signer::new(Signer::from_secret_key(secret_key))?)
    }
}

#[derive(Clone, Debug)]
pub struct PatchTransaction {
    account: AccountId,
    account_data: near_api_types::Account,
    state: Vec<StateRecord>,
}

impl PatchTransaction {
    pub const fn new(account: AccountId, account_data: near_api_types::Account) -> Self {
        Self {
            account,
            account_data,
            state: vec![],
        }
    }

    /// Patch the access keys of an account. This will add or overwrite the current access key
    /// contained in sandbox with the access key we specify.
    pub fn access_key(mut self, public_key: PublicKey, access_key: AccessKey) -> Self {
        self.state.push(StateRecord::AccessKey {
            account_id: self.account.clone(),
            public_key,
            access_key,
        });
        self
    }

    /// Patch the access keys of an account. This will add or overwrite the current access keys
    /// contained in sandbox with a list of access keys we specify.
    ///
    /// Similar to [`PatchTransaction::access_key`], but allows us to specify multiple access keys
    pub fn access_keys<I>(mut self, access_keys: I) -> Self
    where
        I: IntoIterator<Item = (PublicKey, AccessKey)>,
    {
        self.state
            .extend(access_keys.into_iter().map(|(public_key, access_key)| {
                StateRecord::AccessKey {
                    account_id: self.account.clone(),
                    public_key,
                    access_key,
                }
            }));

        self
    }

    /// Sets the code for this account. This will overwrite the current code contained in the account.
    /// Note that if a patch for [`Self::account`] is specified, the code hash
    /// in those will be overwritten with the code hash of the code we specify here.
    pub fn code(mut self, wasm_bytes: &[u8]) -> Self {
        self.account_data.contract_state = ContractState::LocalHash(CryptoHash::hash(wasm_bytes));
        self.state.push(StateRecord::Contract {
            account_id: self.account.clone(),
            code: wasm_bytes.to_vec(),
        });
        self
    }

    /// Patch state into the sandbox network, given a prefix key and value. This will allow us
    /// to set contract state that we have acquired in some manner, where we are able to test
    /// random cases that are hard to come up naturally as state evolves.
    pub fn state(mut self, item: StateItem) -> Self {
        self.state.push(StateRecord::Data {
            account_id: self.account.clone(),
            data_key: item.key,
            value: item.value,
        });
        self
    }

    /// Patch a series of states into the sandbox network. Similar to [`PatchTransaction::state`],
    /// but allows us to specify multiple state patches at once.
    pub fn states<I>(mut self, states: I) -> Self
    where
        I: IntoIterator<Item = StateItem>,
    {
        self.state
            .extend(states.into_iter().map(|v| StateRecord::Data {
                account_id: self.account.clone(),
                data_key: v.key,
                value: v.value,
            }));

        self
    }

    /// Post the state patch to the sandbox network.
    pub async fn post(&self, network: &NetworkConfig) -> Result<(), SandboxError> {
        let mut state = vec![StateRecord::Account {
            account_id: self.account.clone(),
            account: self.account_data.clone(),
        }];

        state.extend(self.state.clone());

        let rpc: SimpleSandboxRpc = SimpleSandboxRpc {
            action: SandboxAction::PatchState(state),
        };

        RequestBuilder::new(rpc.clone(), (), rpc)
            .fetch_from(network)
            .await?;

        Ok(())
    }
}
