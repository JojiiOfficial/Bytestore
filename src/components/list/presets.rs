use super::List;

pub type ListU8<B> = List<B, u8, 1>;
pub type ListI8<B> = List<B, i8, 1>;
pub type ListU16<B> = List<B, u16, 2>;
pub type ListI16<B> = List<B, i16, 2>;
pub type ListU32<B> = List<B, u32, 4>;
pub type ListI32<B> = List<B, i32, 4>;
pub type ListU32Opt<B> = List<B, Option<u32>, 5>;
pub type ListUsize<B> = List<B, usize, 8>;
pub type ListIsize<B> = List<B, isize, 8>;
pub type ListF32<B> = List<B, f32, 4>;
pub type ListF64<B> = List<B, f64, 8>;
pub type ListBool<B> = List<B, bool, 1>;
pub type ListChar<B> = List<B, char, 3>;
