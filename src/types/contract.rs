use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
