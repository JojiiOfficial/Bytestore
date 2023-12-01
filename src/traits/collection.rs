use crate::Result;

/// Trait defining common behavior for components that can be accessed like a list.
pub trait Collection<T> {
    /// The type used to iterate over the collections items.
    type Iter<'a>: IntoIterator<Item = T> + 'a
    where
        Self: 'a;

    type Iterator: IntoIterator<Item = T>;

    /// Gets a single value of a collection by index.
    fn get(&self, index: usize) -> Result<T>;

    /// Iterates over the collection.
    fn iter(&self) -> Self::Iter<'_>;

    /// Owned iterator
    fn into_iter(self) -> Self::Iterator;
}

pub trait GrowableCollection<T>: Collection<T> + Extend<T> {
    /// Pushes a value at the end of a collection.
    fn push(&mut self, item: T) -> Result<()>;
}
