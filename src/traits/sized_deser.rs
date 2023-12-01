/// Trait for sized deserialization.
pub trait SizedDeser<const N: usize>
    where
        Self: Sized,
{
    /// Returns the amount of bytes this value takes to de/serialize. Should never be overwritten!
    #[inline]
    fn size() -> usize {
        N
    }

    fn from_bytes(bytes: [u8; N]) -> Self;

    fn to_bytes(&self) -> [u8; N];
}

#[macro_export]
macro_rules! impl_num {
    ($e:ty, $n:expr) => {
        impl SizedDeser<$n> for $e {
            #[inline(always)]
            fn from_bytes(bytes: [u8; $n]) -> Self {
                <$e>::from_be_bytes(bytes)
            }

            #[inline(always)]
            fn to_bytes(&self) -> [u8; $n] {
                self.to_be_bytes()
            }
        }
    };
}

impl_num!(u8, 1);
impl_num!(i8, 1);
impl_num!(u16, 2);
impl_num!(i16, 2);
impl_num!(u32, 4);
impl_num!(i32, 4);
impl_num!(f32, 4);
impl_num!(u64, 8);
impl_num!(i64, 8);
impl_num!(usize, 8);
impl_num!(isize, 8);
impl_num!(f64, 8);
impl_num!(u128, 16);
impl_num!(i128, 16);

impl SizedDeser<1> for bool {
    #[inline]
    fn from_bytes(bytes: [u8; 1]) -> Self {
        bytes[0] != 0
    }

    #[inline]
    fn to_bytes(&self) -> [u8; 1] {
        [*self as u8]
    }
}

impl SizedDeser<4> for char {
    #[inline]
    fn from_bytes(bytes: [u8; 4]) -> Self {
        char::from_u32(u32::from_be_bytes(bytes)).expect("Not encoded with to_bytes")
    }

    #[inline]
    fn to_bytes(&self) -> [u8; 4] {
        ((*self) as u32).to_be_bytes()
    }
}
