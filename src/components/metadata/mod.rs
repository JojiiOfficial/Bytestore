use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::deser::serialize_impl;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::Error;
use crate::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

pub struct Metadata<B, T, const N: usize> {
    backend: B,
    metadata: T,
    _pt: PhantomData<T>,
}

impl<B, T, const N: usize> Creatable<B> for Metadata<B, T, N>
where
    B: GrowableBackend,
    T: Serialize + Default,
{
    fn with_capacity(mut backend: B, _: usize) -> Result<Self> {
        assert!(N > 0);

        if backend.capacity() < N {
            backend.grow_to(N)?;
        }

        let default = T::default();
        let enc = serialize_impl(&default)?;
        if enc.len() != N {
            return Err(Error::OutOfBounds);
        }

        backend.push(&enc)?;

        Ok(Self {
            backend,
            metadata: default,
            _pt: PhantomData,
        })
    }
}

impl<B, T, const N: usize> Initiable<B> for Metadata<B, T, N>
where
    B: Backend,
    T: DeserializeOwned,
{
    fn init(backend: B) -> Result<Self> {
        assert!(N > 0);
        let data: T = backend.get_t(0, N)?;
        Ok(Self {
            backend,
            metadata: data,
            _pt: PhantomData,
        })
    }
}

impl<B, T, const N: usize> Metadata<B, T, N> {
    /// Reurns the metadata.
    #[inline]
    pub fn get(&self) -> &T {
        &self.metadata
    }
}

impl<B, T, const N: usize> Metadata<B, T, N>
where
    T: Serialize,
    B: Backend,
{
    /// Reurns the metadata.
    #[inline]
    pub fn set(&mut self, new: T) -> Result<()> {
        let data = serialize_impl(&new)?;
        if data.len() != N {
            return Err(Error::OutOfBounds);
        }
        self.backend.replace_same_len(0, &data)?;
        self.metadata = new;
        Ok(())
    }
}
