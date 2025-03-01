use near_contract_standards::{
    fungible_token::metadata::FungibleTokenMetadata,
    non_fungible_token::{metadata::NFTContractMetadata, Token},
};
use near_primitives::{
    action::{Action, TransferAction},
    types::{AccountId, BlockReference},
};
use near_sdk::json_types::U128;
use near_token::NearToken;
use serde_json::json;

use crate::{
    common::{
        query::{
            AccountViewHandler, CallResultHandler, MultiQueryBuilder, MultiQueryHandler,
            PostprocessHandler, QueryBuilder, SimpleQuery,
        },
        send::Transactionable,
    },
    contract::Contract,
    errors::{BuilderError, FTValidatorError, ValidationError},
    transactions::{ConstructTransaction, TransactionWithSign},
    types::{
        tokens::{FTBalance, UserBalance, STORAGE_COST_PER_BYTE},
        transactions::PrepopulateTransaction,
    },
    Data, NetworkConfig, StorageDeposit,
};

type Result<T> = core::result::Result<T, BuilderError>;

// This is not too long as most of the size is a links to the docs
#[allow(clippy::too_long_first_doc_paragraph)]
/// A wrapper struct that simplifies interactions with
/// [NEAR](https://docs.near.org/concepts/basics/tokens),
/// [FT](https://docs.near.org/build/primitives/ft),
/// [NFT](https://docs.near.org/build/primitives/nft)
///
/// This struct provides convenient methods to interact with different types of tokens on NEAR Protocol:
/// - [Native NEAR](https://docs.near.org/concepts/basics/tokens) token operations
/// - Fungible Token - [Documentation and examples](https://docs.near.org/build/primitives/ft), [NEP-141](https://github.com/near/NEPs/blob/master/neps/nep-0141.md)    
/// - Non-Fungible Token - [Documentation and examples](https://docs.near.org/build/primitives/nft), [NEP-171](https://github.com/near/NEPs/blob/master/neps/nep-0171.md)
///
/// ## Examples
///
/// ### Fungible Token Operations
/// ```
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let bob_tokens = Tokens::account("bob.testnet".parse()?);
///
/// // Check FT balance
/// let balance = bob_tokens.ft_balance("usdt.tether-token.near".parse()?)?.fetch_from_mainnet().await?;
/// println!("Bob balance: {}", balance);
///
/// // Transfer FT tokens
/// bob_tokens.send_to("alice.testnet".parse()?)
///     .ft(
///         "usdt.tether-token.near".parse()?,
///         USDT_BALANCE.with_whole_amount(100)
///     )?
///     .with_signer(Signer::new(Signer::from_ledger())?)
///     .send_to_mainnet()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// ### NFT Operations
/// ```
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let alice_tokens = Tokens::account("alice.testnet".parse()?);
///
/// // Check NFT assets
/// let tokens = alice_tokens.nft_assets("nft-contract.testnet".parse()?)?.fetch_from_testnet().await?;
/// println!("NFT count: {}", tokens.data.len());
///
/// // Transfer NFT
/// alice_tokens.send_to("bob.testnet".parse()?)
///     .nft("nft-contract.testnet".parse()?, "token-id".to_string())?
///     .with_signer(Signer::new(Signer::from_ledger())?)
///     .send_to_testnet()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// ### NEAR Token Operations
/// ```
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let alice_account = Tokens::account("alice.testnet".parse()?);
///
/// // Check NEAR balance
/// let balance = alice_account.near_balance().fetch_from_testnet().await?;
/// println!("NEAR balance: {}", balance.total);
///
/// // Send NEAR
/// alice_account.send_to("bob.testnet".parse()?)
///     .near(NearToken::from_near(1))
///     .with_signer(Signer::new(Signer::from_ledger())?)
///     .send_to_testnet()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Tokens {
    account_id: AccountId,
}

impl Tokens {
    pub const fn account(account_id: AccountId) -> Self {
        Self { account_id }
    }

    /// Fetches the total NEAR balance ([UserBalance]) of the account.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_tokens = Tokens::account("alice.testnet".parse()?);
    /// let balance = alice_tokens.near_balance().fetch_from_testnet().await?;
    /// println!("Alice's NEAR balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn near_balance(
        &self,
    ) -> QueryBuilder<PostprocessHandler<UserBalance, AccountViewHandler>> {
        let request = near_primitives::views::QueryRequest::ViewAccount {
            account_id: self.account_id.clone(),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            AccountViewHandler,
        )
        .map(|account| {
            let account = account.data;
            let total = NearToken::from_yoctonear(account.amount);
            let storage_locked = NearToken::from_yoctonear(
                account.storage_usage as u128 * STORAGE_COST_PER_BYTE.as_yoctonear(),
            );
            let locked = NearToken::from_yoctonear(account.locked);
            let storage_usage = account.storage_usage;
            UserBalance {
                total,
                storage_locked,
                storage_usage,
                locked,
            }
        })
    }

    /// Prepares a new contract query (`nft_metadata`) for fetching the NFT metadata ([NFTContractMetadata]).
    ///
    /// The function depends that the contract implements [`NEP-171`](https://nomicon.io/Standards/Tokens/NonFungibleToken/Core#nep-171)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let metadata = Tokens::nft_metadata("nft-contract.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("NFT metadata: {:?}", metadata);
    /// # Ok(())
    /// # }
    /// ```
    pub fn nft_metadata(
        contract_id: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<NFTContractMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("nft_metadata", ())?
            .read_only())
    }

    /// Prepares a new contract query (`nft_tokens_for_owner`) for fetching the NFT assets of the account ([Vec]<[Token]>).
    ///
    /// The function depends that the contract implements [`NEP-171`](https://nomicon.io/Standards/Tokens/NonFungibleToken/Core#nep-171)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_tokens = Tokens::account("alice.testnet".parse()?);
    /// let alice_assets = alice_tokens.nft_assets("nft-contract.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Alice's NFT assets: {:?}", alice_assets);
    /// # Ok(())
    /// # }
    /// ```
    pub fn nft_assets(
        &self,
        nft_contract: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<Vec<Token>>>> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_tokens_for_owner",
                json!({
                    "account_id": self.account_id.to_string(),
                }),
            )?
            .read_only())
    }

    /// Prepares a new contract query (`ft_metadata`) for fetching the FT metadata ([FungibleTokenMetadata]).
    ///
    /// The function depends that the contract implements [`NEP-141`](https://nomicon.io/Standards/Tokens/FungibleToken/Core#nep-141)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let metadata = Tokens::ft_metadata("usdt.tether-token.near".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?
    ///     .data;
    /// println!("FT metadata: {} {}", metadata.name, metadata.symbol);
    /// # Ok(())
    /// # }
    /// ```
    pub fn ft_metadata(
        contract_id: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<FungibleTokenMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("ft_metadata", ())?
            .read_only())
    }

    /// Prepares a new contract query (`ft_balance_of`, `ft_metadata`) for fetching the [FTBalance] of the account.
    ///
    /// This query is a multi-query, meaning it will fetch the FT metadata and the FT balance of the account.
    /// The result is then postprocessed to create a `FTBalance` instance.
    ///
    /// The function depends that the contract implements [`NEP-141`](https://nomicon.io/Standards/Tokens/FungibleToken/Core#nep-141)
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_usdt_balance = Tokens::account("alice.near".parse()?)
    ///     .ft_balance("usdt.tether-token.near".parse()?)?
    ///     .fetch_from_mainnet()
    ///     .await?;
    /// println!("Alice's USDT balance: {}", alice_usdt_balance);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::complexity)]
    pub fn ft_balance(
        &self,
        ft_contract: AccountId,
    ) -> Result<
        MultiQueryBuilder<
            PostprocessHandler<
                FTBalance,
                MultiQueryHandler<(
                    CallResultHandler<FungibleTokenMetadata>,
                    CallResultHandler<U128>,
                )>,
            >,
        >,
    > {
        let handler = MultiQueryHandler::new((
            CallResultHandler::<FungibleTokenMetadata>::new(),
            CallResultHandler::default(),
        ));
        let multiquery = MultiQueryBuilder::new(handler, BlockReference::latest())
            .add_query_builder(Self::ft_metadata(ft_contract.clone())?)
            .add_query_builder(
                Contract(ft_contract)
                    .call_function(
                        "ft_balance_of",
                        json!({
                            "account_id": self.account_id.clone()
                        }),
                    )?
                    .read_only::<()>(),
            )
            .map(
                |(metadata, amount): (Data<FungibleTokenMetadata>, Data<U128>)| {
                    FTBalance::with_decimals(metadata.data.decimals).with_amount(amount.data.0)
                },
            );
        Ok(multiquery)
    }

    /// Prepares a new transaction builder for sending tokens to another account.
    ///
    /// This builder is used to construct transactions for sending NEAR, FT, and NFT tokens.
    ///
    /// ## Sending NEAR
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_tokens = Tokens::account("alice.near".parse()?);
    ///
    /// let result: near_primitives::views::FinalExecutionOutcomeView = alice_tokens.send_to("bob.near".parse()?)
    ///     .near(NearToken::from_near(1))
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_mainnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Sending FT
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_tokens = Tokens::account("alice.near".parse()?);
    ///
    /// let result: near_primitives::views::FinalExecutionOutcomeView = alice_tokens.send_to("bob.near".parse()?)
    ///     .ft("usdt.tether-token.near".parse()?, USDT_BALANCE.with_whole_amount(100))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_mainnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Sending NFT
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let alice_tokens = Tokens::account("alice.near".parse()?);
    ///
    /// let result: near_primitives::views::FinalExecutionOutcomeView = alice_tokens.send_to("bob.near".parse()?)
    ///     .nft("nft-contract.testnet".parse()?, "token-id".to_string())?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_to(&self, receiver_id: AccountId) -> SendToBuilder {
        SendToBuilder {
            from: self.account_id.clone(),
            receiver_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendToBuilder {
    from: AccountId,
    receiver_id: AccountId,
}

impl SendToBuilder {
    /// Prepares a new transaction for sending NEAR tokens to another account.
    pub fn near(self, amount: NearToken) -> ConstructTransaction {
        ConstructTransaction::new(self.from, self.receiver_id).add_action(Action::Transfer(
            TransferAction {
                deposit: amount.as_yoctonear(),
            },
        ))
    }

    /// Prepares a new transaction contract call (`ft_transfer`, `ft_metadata`, `storage_balance_of`, `storage_deposit`) for sending FT tokens to another account.
    ///
    /// Please note that if the receiver does not have enough storage, we will automatically deposit 100 milliNEAR for storage from
    /// the sender.
    ///
    /// The provided function depends that the contract implements [`NEP-141`](https://nomicon.io/Standards/Tokens/FungibleToken/Core#nep-141)
    pub fn ft(
        self,
        ft_contract: AccountId,
        amount: FTBalance,
    ) -> Result<TransactionWithSign<FTTransactionable>> {
        let tr = Contract(ft_contract)
            .call_function(
                "ft_transfer",
                json!({
                    "receiver_id": self.receiver_id,
                    "amount": U128(amount.amount()),
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(self.from);

        Ok(TransactionWithSign {
            tx: FTTransactionable {
                receiver: self.receiver_id,
                prepopulated: tr.tr,
                decimals: amount.decimals(),
            },
        })
    }

    /// Prepares a new transaction contract call (`nft_transfer`) for sending NFT tokens to another account.
    ///
    /// The provided function depends that the contract implements [`NEP-171`](https://nomicon.io/Standards/Tokens/NonFungibleToken/Core#nep-171)
    pub fn nft(self, nft_contract: AccountId, token_id: String) -> Result<ConstructTransaction> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_transfer",
                json!({
                    "receiver_id": self.receiver_id,
                    "token_id": token_id
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(self.from))
    }
}

/// The structs validates the decimals correctness on runtime level before
/// sending the ft tokens as well as deposits 100 milliNear of the deposit if
/// the receiver doesn't have any allocated storage in the provided FT contract
#[derive(Clone, Debug)]
pub struct FTTransactionable {
    prepopulated: PrepopulateTransaction,
    receiver: AccountId,
    decimals: u8,
}

impl FTTransactionable {
    pub async fn check_decimals(
        &self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        let metadata = Tokens::ft_metadata(self.prepopulated.receiver_id.clone())?;

        let metadata = metadata
            .fetch_from(network)
            .await
            .map_err(|_| FTValidatorError::NoMetadata)?;
        if metadata.data.decimals != self.decimals {
            Err(FTValidatorError::DecimalsMismatch {
                expected: metadata.data.decimals,
                got: self.decimals,
            })?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Transactionable for FTTransactionable {
    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    async fn validate_with_network(
        &self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        self.check_decimals(network).await?;

        let storage_balance = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
            .view_account_storage(self.receiver.clone())?
            .fetch_from(network)
            .await?;

        if storage_balance.data.is_none() {
            Err(FTValidatorError::StorageDepositNeeded)?;
        }

        Ok(())
    }

    async fn edit_with_network(
        &mut self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        self.check_decimals(network).await?;

        let storage_balance = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
            .view_account_storage(self.receiver.clone())?
            .fetch_from(network)
            .await?;

        if storage_balance.data.is_none() {
            let mut action = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
                .deposit(self.receiver.clone(), NearToken::from_millinear(100))?
                .with_signer_account(self.prepopulated.signer_id.clone())
                .tr
                .actions;
            action.append(&mut self.prepopulated.actions);
            self.prepopulated.actions = action;
        }
        Ok(())
    }
}
