use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait Deser: Serialize + DeserializeOwned {}

impl<T> Deser for T where T: Serialize + DeserializeOwned {}
