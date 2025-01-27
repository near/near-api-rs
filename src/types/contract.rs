use serde::{Deserialize, Serialize};

/// The struct provides information about deployed contract's source code and supported standards.
///
/// Contract source metadata follows [NEP-330 standard](https://nomicon.io/Standards/SourceMetadata) for smart contract verification
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct ContractSourceMetadata {
    /// Optional version identifier, typically a commit hash or semantic version
    ///
    /// Examples:
    /// - Git commit: "39f2d2646f2f60e18ab53337501370dc02a5661c"
    /// - Semantic version: "1.0.0"
    pub version: Option<String>,

    // cSpell::ignore bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq
    /// Optional URL to source code repository or IPFS CID
    ///
    /// Examples:
    /// - GitHub URL: "<https://github.com/near-examples/nft-tutorial>"
    /// - IPFS CID: "bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq"
    pub link: Option<String>,

    /// List of supported NEAR standards (NEPs) with their versions
    ///
    /// Should include NEP-330 itself if implemented:
    /// `Standard { standard: "nep330".into(), version: "1.1.0".into() }`
    pub standards: Vec<Standard>,
}

/// NEAR Standard implementation descriptor following [NEP-330](https://nomicon.io/Standards/SourceMetadata)    
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct Standard {
    /// Standard name in lowercase NEP format
    ///
    /// Example: "nep141" for fungible tokens
    pub standard: String,

    /// Implemented standard version using semantic versioning
    ///
    /// Example: "1.0.0" for initial release
    pub version: String,
}
