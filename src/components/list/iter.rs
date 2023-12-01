use super::List;
use crate::backend::Backend;
use serde::de::DeserializeOwned;

/// An iterator over items of a `List`
pub struct ListIter<'a, B, T, const N: usize> {
    list: &'a List<B, T, N>,
    pos: usize,
    pos_back: usize,
}

impl<'a, B, T, const N: usize> ListIter<'a, B, T, N> {
    #[inline]
    pub(super) fn new(list: &'a List<B, T, N>) -> Self {
        Self {
            pos: 0,
            pos_back: list.len(),
            list,
        }
    }
}

impl<'a, B, T, const N: usize> ListIter<'a, B, T, N>
where
    B: Backend,
    T: DeserializeOwned,
{
    #[inline]
    fn item_at_unchecked(&self, pos: usize) -> T {
        self.list.get(pos).expect("Failed to lad item")
    }
}

impl<'a, B, T, const N: usize> Iterator for ListIter<'a, B, T, N>
where
    B: Backend,
    T: DeserializeOwned,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.list.len() {
            return None;
        }
        let item = self.item_at_unchecked(self.pos);
        self.pos += 1;
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.list.len(), Some(self.list.len()))
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.list.len()
    }

    #[inline]
    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if self.list.is_empty() {
            return None;
        }
        Some(self.item_at_unchecked(self.list.len() - 1))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.pos += n;
        self.next()
    }
}

impl<'a, B, T, const N: usize> DoubleEndedIterator for ListIter<'a, B, T, N>
where
    B: Backend,
    T: DeserializeOwned,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.pos_back == 0 {
            return None;
        }
        self.pos_back -= 1;
        Some(self.item_at_unchecked(self.pos_back))
    }
}

#[cfg(test)]
mod test {
    use crate::components::list::ListU32;

    #[test]
    fn list_iter() {
        let list: ListU32<_> = (0..1000).step_by(13).collect();
        let mut iter = list.iter();
        for i in (0..1000u32).step_by(13) {
            assert_eq!(iter.next(), Some(i));
        }
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn list_iter_back() {
        let list: ListU32<_> = (0..1000).step_by(13).collect();
        let mut iter = list.iter().rev();
        for i in (0..1000u32).step_by(13).rev() {
            assert_eq!(iter.next(), Some(i));
        }
        assert_eq!(iter.next(), None);
    }
}
