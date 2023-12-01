use crate::Result;

pub trait Initiable<B>
where
    Self: Sized,
{
    fn init(backend: B) -> Result<Self>;
}
