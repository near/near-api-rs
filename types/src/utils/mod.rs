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
