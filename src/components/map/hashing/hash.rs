use crate::traits::sized_deser::SizedDeser;

pub trait Hash {
    fn hash(&self) -> u64;
}

impl<'a, const N: usize> Hash for &'a [char; N] {
    #[inline]
    fn hash(&self) -> u64 {
        (&self[..]).hash()
    }
}

impl<const N: usize> Hash for [char; N] {
    #[inline]
    fn hash(&self) -> u64 {
        (&self[..]).hash()
    }
}

impl Hash for &[char] {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash_it(self.iter().flat_map(|i| i.to_bytes()))
    }
}

impl Hash for &[u8] {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash(self)
    }
}

impl Hash for Vec<u8> {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash(self)
    }
}

impl Hash for String {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash(self.as_bytes())
    }
}

impl Hash for &String {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash(self.as_bytes())
    }
}

impl Hash for &str {
    #[inline]
    fn hash(&self) -> u64 {
        fnv_hash(self.as_bytes())
    }
}

impl Hash for u8 {
    #[inline]
    fn hash(&self) -> u64 {
        (*self as u64).hash()
    }
}

impl Hash for u16 {
    #[inline]
    fn hash(&self) -> u64 {
        (*self as u64).hash()
    }
}

impl Hash for u32 {
    #[inline]
    fn hash(&self) -> u64 {
        (*self as u64).hash()
    }
}

impl Hash for u64 {
    #[inline]
    fn hash(&self) -> u64 {
        *self
    }
}

impl Hash for char {
    #[inline]
    fn hash(&self) -> u64 {
        (*self) as u64
    }
}

const INIT_V: u64 = 14695981039346656037;
const PRIME: u64 = 1099511628211;

#[inline]
pub fn fnv_hash(b: &[u8]) -> u64 {
    fnv_hash_it(b.iter().copied())
}

#[inline]
pub fn fnv_hash_it<I>(i: I) -> u64 where I: IntoIterator<Item=u8> {
    i.into_iter()
        .fold(INIT_V, |h, e| (h ^ (e as u64)).wrapping_mul(PRIME))
}
