use crate::error::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[inline]
pub fn serialize_impl<T: Serialize>(item: &T) -> Result<Vec<u8>, Error> {
    Ok(bitcode::serialize(item)?)
}

#[inline]
pub fn deserialize_impl<T: DeserializeOwned>(data: &[u8]) -> Result<T, Error> {
    Ok(bitcode::deserialize(data)?)
}
