// mostly taken from the suffix_array crate but with modifications

use std::{
    ops::{Deref, Range},
    slice::from_raw_parts_mut,
};

use cdivsufsort::sort_in_place as divsufsort;

/// Maximum length of the input string.
pub const MAX_LENGTH: usize = std::i32::MAX as usize;

/// Wrapper of the underlying suffix array construction algorithm.
pub fn saca(s: &[u8], sa: &mut [u32]) {
    assert!(s.len() <= MAX_LENGTH);
    assert_eq!(s.len() + 1, sa.len());

    sa[0] = s.len() as u32;
    divsufsort(s, as_signed_integer_slice(&mut sa[1..]));
}

fn as_signed_integer_slice(sa: &mut [u32]) -> &mut [i32] {
    unsafe {
        let len = sa.len();
        let data = sa.as_mut_ptr() as *mut i32;
        from_raw_parts_mut(data, len)
    }
}

#[derive(Clone)]
pub struct SuffixArray {
    sa: Vec<u32>,
    bkt: Vec<u32>,
}

pub fn compute_buckets(text: &[u8], sa: &[u32]) -> Vec<u32> {
    // the layout is [$; (0, $), (0, 0), ..., (0, 255); ...; (255, $), (255, 0), ..., (255, 255)]
    let mut bkt = vec![0; 256 * 257 + 1];

    // count occurrences.
    bkt[0] = 1;
    if text.len() > 0 {
        for i in 0..text.len() - 1 {
            let c0 = unsafe { *text.get_unchecked(i) };
            let c1 = unsafe { *text.get_unchecked(i + 1) };
            let idx = (c0 as usize * 257) + (c1 as usize + 1) + 1;
            bkt[idx] += 1;
        }
        let c0 = unsafe { *text.get_unchecked(text.len() - 1) };
        let idx = (c0 as usize * 257) + 1;
        bkt[idx] += 1;
    }

    // store the right boundaries of each bucket.
    let mut sum = 0;
    for p in bkt.iter_mut() {
        sum += *p;
        *p = sum;
    }
    bkt
}

impl SuffixArray {
    pub fn new(text: &[u8]) -> Self {
        let mut sa = vec![0; text.len() + 1];
        saca(text, &mut sa[..]);
        SuffixArray {
            bkt: compute_buckets(text, &sa),
            sa,
        }
    }

    pub fn len(&self) -> usize {
        self.sa.len()
    }

    #[inline]
    fn get_bucket(&self, pat: &[u8]) -> Range<usize> {
        let bkt = &self.bkt;
        if pat.len() > 1 {
            // sub-bucket (c0, c1).
            let c0 = pat[0];
            let c1 = pat[1];
            let idx = (c0 as usize * 257) + (c1 as usize + 1) + 1;
            bkt[idx - 1] as usize..bkt[idx] as usize
        } else if pat.len() == 1 {
            // top-level bucket (c0, $)..=(c0, 255).
            let c0 = pat[0];
            let start_idx = c0 as usize * 257;
            let end_idx = start_idx + 257;
            bkt[start_idx] as usize..bkt[end_idx] as usize
        } else {
            // the sentinel bucket.
            0..1
        }
    }

    pub(crate) fn refine_bounds(
        &self,
        bounds: Range<usize>,
        pat_sym: u8,
        offset: usize,
        text: &[u8],
    ) -> Option<Range<usize>> {
        // refine left bound
        let mut new_left = bounds.start;
        let sa_range = &self.sa[bounds];
        while !sa_range.is_empty() {
            let mid_pos = sa_range.len() / 2;
            let mid = sa_range[mid_pos] as usize;
            let dict_sym = text[mid + offset];
            if dict_sym < pat_sym {
                sa_range = &sa_range[mid_pos + 1..];
                new_left += mid_pos + 1;
            } else {
                sa_range = &sa_range[..mid];
            }
        }

        // refine right bound
        let mut new_right = bounds.end;
        let sa_range = &self.sa[bounds];
        while !sa_range.is_empty() {
            let mid_pos = sa_range.len() / 2;
            let mid = sa_range[mid_pos] as usize;
            let dict_sym = text[mid + offset];
            if dict_sym <= pat_sym {
                sa_range = &sa_range[mid_pos + 1..];
                new_right += mid_pos + 1;
            } else {
                sa_range = &sa_range[..mid];
            }
        }

        let dict_sym = text[sa_range[0] as usize + offset];
        if dict_sym != pat_sym {
            new_right -= 1;
        }

        if new_left <= new_right {
            Some(Range {
                start: new_left,
                end: new_right,
            })
        } else {
            None
        }
    }
}

impl Deref for SuffixArray {
    type Target = Vec<u32>;

    fn deref(&self) -> &Self::Target {
        &self.sa
    }
}
