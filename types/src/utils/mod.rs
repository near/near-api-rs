/// Convenience module to allow annotating a serde structure as base64 bytes.
pub mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer, de};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(s.as_str())
            .map_err(de::Error::custom)
    }
}

pub mod near_gas_as_u64 {
    use near_gas::NearGas;
    use serde::Serializer;

    pub fn serialize<S>(value: &NearGas, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(value.as_gas())
    }
}
