use crate::backend::Backend;
use crate::{Error, Result};

/// A `Backend` extending trait to define backends that can dynamically grow in size.
pub trait GrowableBackend: Backend {
    /// Function needed to be implemented to support resizing for backends.
    fn resize_impl(&mut self, _new_size: usize, growing: bool) -> Result<()>;

    #[inline]
    fn grow(&mut self, size: usize) -> Result<()> {
        self.resize(size as isize)
    }

    #[inline]
    fn shrink(&mut self, size: usize) -> Result<()> {
        self.resize(-(size as isize))
    }

    /// Shrinks the backend and truncates all free bytes.
    #[inline]
    fn shrink_to_fit(&mut self) -> Result<()> {
        self.shrink(self.free())
    }

    fn grow_to(&mut self, size: usize) -> Result<()> {
        // Total backend len with header
        let len = self.data().len();
        if size <= (len.saturating_sub(self.first_index())) {
            return Ok(());
        }

        let diff = (size + self.first_index()) - len;
        self.grow(diff)
    }

    fn resize(&mut self, delta: isize) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }

        let data_len = self.data().len();

        if delta < 0 && delta.unsigned_abs() > data_len {
            return Err(Error::OutOfBounds);
        }

        let new_size = (self.data().len() as isize)
            .checked_add(delta)
            .expect("Overflowing int") as usize;

        if new_size < self.last_index() {
            return Err(Error::OutOfBounds);
        }

        self.resize_impl(new_size, delta > 0)
    }
}

impl<T> GrowableBackend for &mut T
where
    T: GrowableBackend,
{
    #[inline]
    fn resize_impl(&mut self, new_size: usize, growing: bool) -> Result<()> {
        (**self).resize_impl(new_size, growing)
    }
}
