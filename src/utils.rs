use std::ops::Range;

/// Returns `true` if the ranges overlap
#[inline]
pub fn ranges_overlap<O: Ord>(a: &Range<O>, b: &Range<O>) -> bool {
    a.start <= b.end && a.end >= b.start
}

/// Smallest number x so that `2^x >= len`.
pub fn smallest_two_power_for(len: usize) -> u32 {
    let mut req_len = len.max(1).ilog2().max(1);

    if !len.is_power_of_two() {
        req_len += 1;
    }

    req_len
}
