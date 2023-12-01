use std::marker::PhantomData;

pub trait HashFn {
    fn f(hash: u64, pos: usize, max: usize) -> u64;
}

pub struct LinearProbing;

impl HashFn for LinearProbing {
    #[inline]
    fn f(hash: u64, pos: usize, max: usize) -> u64 {
        let pos = pos as u64;
        let max = max as u64;
        ((hash % max) + (pos % max)) % max
    }
}

pub struct PALinearProbing;

impl HashFn for PALinearProbing {
    #[inline]
    fn f(hash: u64, pos: usize, max: usize) -> u64 {
        let pos = pos as u64;
        let max = max as u64;
        ((hash % max) + (pos % max)) % max
    }
}

pub struct QuadraticProbing;

impl HashFn for QuadraticProbing {
    #[inline]
    fn f(hash: u64, pos: usize, max: usize) -> u64 {
        let i = pos as u64;
        let max = max as u64;
        let c1 = 65537 % max;
        let c2 = 16411 % max;
        (hash + ((c1 * (i % max)) % max) + ((c2 * (i.pow(2) % max)) % max)) % max
        // ((hash % max) + i.pow(2)) % max
    }
}

pub struct DoubleHashing<F, S> {
    p: PhantomData<(F, S)>,
}

impl<F, S> HashFn for DoubleHashing<F, S>
    where
        F: HashFn,
        S: HashFn,
{
    #[inline]
    fn f(hash: u64, pos: usize, max: usize) -> u64 {
        let first = F::f(hash, 1, max);
        let second = S::f(hash, pos, max);

        let max = max as u64;

        (first + second) % max
    }
}
