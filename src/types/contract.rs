pub use build_info::BuildInfo;
use serde::{Deserialize, Serialize};

/// The struct provides information about deployed contract's source code and supported standards.
///
/// Contract source metadata follows [**NEP-330 standard**](https://github.com/near/NEPs/blob/master/neps/nep-0330.md) for smart contract verification
/// 
/// See documentation of [`crate::Contract::contract_source_metadata`] on how to query this for a contract via this crate
// `rustdoc` clearly lacks functionality of automatic backlinks within a single crate
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct ContractSourceMetadata {
    /// Optional version identifier, typically a semantic version
    ///
    /// **NOTE**:
    /// As of **NEP-330** standard version **1.2.0**
    /// this field may or may not be consistent with [`Self::link`] and with [`BuildInfo::source_code_snapshot`], but only [`BuildInfo::source_code_snapshot`] defines source code for formal reproducibility verification, and [`Self::link`] and [`Self::version`] do not
    /// 
    /// ## Examples:
    /// 
    /// ```rust,no_run
    /// # let version: Option<String> =
    /// // Semantic version 
    /// Some("1.0.0".into())
    /// # ;
    /// ```
    pub version: Option<String>,

    // cSpell::ignore bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq
    /// Optional URL to source code repository/tree
    ///
    /// **NOTE**:
    /// As of **NEP-330** standard version **1.2.0**
    /// this field may or may not be consistent with [`Self::version`] and with [`BuildInfo::source_code_snapshot`], but only [`BuildInfo::source_code_snapshot`] defines source code for formal reproducibility verification, and [`Self::link`] and [`Self::version`] do not
    ///
    /// ## Examples:
    /// 
    /// ```rust,no_run
    /// # let link: Option<String> =
    /// // GitHub URL 
    /// Some("https://github.com/near-examples/nft-tutorial".into())
    /// # ;
    /// ```
    /// ```rust,no_run
    /// # let link: Option<String> =
    /// // GitHub URL 
    /// Some("https://github.com/org/repo/tree/8d8a8a0fe86a1d8eb3bce45f04ab1a65fecf5a1b".into())
    /// # ;
    /// ```
    pub link: Option<String>,

    /// List of supported NEAR standards (NEPs) with their versions
    ///
    /// This field is an addition of **1.1.0** **NEP-330** revision
    /// 
    /// ## Examples:
    /// 
    /// This field will always include NEP-330 itself:
    /// ```rust,no_run
    /// # use near_api::types::contract::Standard;
    /// # let link: Vec<Standard> =
    /// // this is always at least 1.1.0
    /// vec![Standard { standard: "nep330".into(), version: "1.1.0".into() }]
    /// # ;
    /// ```
    /// ```rust,no_run
    /// # use near_api::types::contract::Standard;
    /// # let link: Vec<Standard> =
    /// vec![Standard { standard: "nep330".into(), version: "1.2.0".into() }]
    /// # ;
    /// ```
    // it's a guess it was added as 1.1.0 of nep330, [nep330 1.1.0 standard recording](https://www.youtube.com/watch?v=pBLN9UyE6AA) actually discusses nep351
    pub standards: Vec<Standard>,

    /// Optional details that are required for formal contract WASM build reproducibility verification
    /// 
    /// This field is an addition of **1.2.0** **NEP-330** revision
    pub build_info: Option<BuildInfo>,
}

/// NEAR Standard implementation descriptor following [NEP-330](https://github.com/near/NEPs/blob/master/neps/nep-0330.md)    
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct Standard {
    /// Standard name in lowercase NEP format
    ///
    /// ## Examples:
    /// 
    /// ```rust,no_run
    /// # let standard: String =
    /// // for fungible tokens
    /// "nep141".into()
    /// # ;
    /// ```
    pub standard: String,

    /// Implemented standard version using semantic versioning
    ///
    /// ## Examples:
    ///
    /// ```rust,no_run
    /// # let standard: String =
    /// // for initial release
    /// "1.0.0".into()
    /// # ;
    /// ```
    pub version: String,
}

mod build_info {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        /// Reference to a reproducible build environment docker image
        ///
        /// ## Examples:
        ///
        /// ```rust,no_run
        /// # let build_environment: String =  
        ///  "sourcescan/cargo-near:0.13.3-rust-1.84.0@sha256:722198ddb92d1b82cbfcd3a4a9f7fba6fd8715f4d0b5fb236d8725c4883f97de".into()
        /// # ;
        /// ```
        pub build_environment: String,
        /// The exact command that was used to build the contract, with all the flags
        ///
        /// ## Examples:
        ///
        /// ```rust,no_run
        /// # let build_command: Vec<String> =
        /// vec![
        ///     "cargo".into(),
        ///     "near".into(),
        ///     "build".into(),
        ///     "non-reproducible-wasm".into(),
        ///     "--locked".into()
        /// ]
        /// # ;
        /// ```
        pub build_command: Vec<String>,
        /// Relative path to contract crate within the source code
        ///
        /// ## Examples:
        ///
        /// ```rust,no_run
        /// # let contract_path: String =
        /// "near/omni-prover/wormhole-omni-prover-proxy".into()
        /// # ;
        /// ```
        /// ```rust,no_run
        /// # let contract_path: String =
        /// // root of a repo
        /// "".into()
        /// # ;
        /// ```
        pub contract_path: String,
        /// Reference to the source code snapshot that was used to build the contract
        ///
        /// ## Examples:
        ///
        /// ```rust,no_run
        /// # let source_code_snapshot: String =
        /// "git+https://github.com/org/repo?rev=8d8a8a0fe86a1d8eb3bce45f04ab1a65fecf5a1b".into()
        /// # ;
        /// ```
        pub source_code_snapshot: String,
    }
}
