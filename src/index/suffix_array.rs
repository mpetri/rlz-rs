// mostly taken from the suffix_array crate but with modifications

use std::{
    ops::{Deref, Range, RangeInclusive},
    slice::from_raw_parts_mut,
};

use cdivsufsort::sort_in_place as divsufsort;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaMatchRangeInclusive {
    pub start: u32,
    pub end: u32,
}

impl SaMatchRangeInclusive {
    pub fn len(&self) -> usize {
        self.end as usize + 1 - self.start as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn empty() -> Self {
        Self { start: 1, end: 0 }
    }
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

impl std::ops::Index<SaMatchRangeInclusive> for Vec<u32> {
    type Output = [u32];

    fn index(&self, range: SaMatchRangeInclusive) -> &Self::Output {
        &self[range.start as usize..=range.end as usize]
    }
}

impl std::ops::Index<&SaMatchRangeInclusive> for Vec<u32> {
    type Output = [u32];

    fn index(&self, range: &SaMatchRangeInclusive) -> &Self::Output {
        &self[range.start as usize..=range.end as usize]
    }
}

/// Maximum length of the input string.
pub const MAX_LENGTH: usize = std::i32::MAX as usize;

/// Wrapper of the underlying suffix array construction algorithm.
pub fn saca(s: &[u8], sa: &mut [u32]) {
    assert!(s.len() <= MAX_LENGTH);
    divsufsort(s, as_signed_integer_slice(&mut sa[..]));
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

fn compute_buckets(text: &[u8]) -> Vec<u32> {
    let num_uniq_chars: usize = u8::MAX as usize + 1;
    let size_per_char = num_uniq_chars + 1; // 1 for MAX one for uni
    let num_bigram_buckets = num_uniq_chars * size_per_char;
    let num_buckets: usize = num_bigram_buckets + 1; // for 0 offset
    let mut bkt = vec![0; num_buckets];

    if text.is_empty() {
        return bkt;
    }

    // count occurrences.
    for bigram in text.windows(2) {
        let c0 = unsafe { *bigram.get_unchecked(0) } as usize;
        let c1 = unsafe { *bigram.get_unchecked(1) } as usize;
        let first_char_offset = c0 * size_per_char;
        let second_char_offset = c1 + 1;
        let bigram_idx = first_char_offset + second_char_offset + 1;
        bkt[bigram_idx] += 1;
    }
    // window(2) misses the last sym
    let last_sym = *text.last().unwrap() as usize;
    bkt[last_sym * size_per_char + 1] += 1;

    // store the right boundaries of each bucket.
    let mut sum = 0;
    for p in bkt.iter_mut() {
        sum += *p;
        *p = sum;
    }
    bkt
}

#[derive(Clone, PartialEq, Eq, Debug)]

pub(crate) enum BucketMatch {
    NoMatch,
    Match {
        num_matched: usize,
        range: SaMatchRangeInclusive,
    },
}

fn get_bucket(bkt: &[u32], pat: &[u8]) -> BucketMatch {
    let size_per_char = u8::MAX as usize + 2; // 1 for MAX 1 for unigram
    match pat.len() {
        0 => BucketMatch::Match {
            num_matched: 0,
            range: SaMatchRangeInclusive::new(bkt[0], bkt.last().unwrap() - 1),
        },
        1 => {
            // top-level bucket (c0, $)..=(c0, 255).
            let c0 = pat[0];
            let start_idx = c0 as usize * size_per_char;
            let end_idx = start_idx + size_per_char;
            let start = bkt[start_idx];
            if bkt[end_idx] == 0 {
                return BucketMatch::NoMatch;
            }
            let end = bkt[end_idx] - 1;
            if start > end {
                BucketMatch::NoMatch
            } else {
                BucketMatch::Match {
                    num_matched: 1,
                    range: SaMatchRangeInclusive::new(start, end),
                }
            }
        }
        _ => {
            // sub-bucket (c0, c1).
            let c0 = pat[0];
            let c1 = pat[1];
            let idx = (c0 as usize * size_per_char) + (c1 as usize + 1) + 1;
            let start = bkt[idx - 1];
            if bkt[idx] == 0 {
                return BucketMatch::NoMatch;
            }
            let end = bkt[idx] - 1;
            if start > end {
                // bi-gram is empty, return unigram
                return get_bucket(bkt, &pat[..1]);
            }
            BucketMatch::Match {
                num_matched: 2,
                range: SaMatchRangeInclusive::new(start, end),
            }
        }
    }
}

impl SuffixArray {
    pub fn new(text: &[u8]) -> Self {
        let mut sa = vec![0; text.len()];
        saca(text, &mut sa[..]);
        SuffixArray {
            bkt: compute_buckets(text),
            sa,
        }
    }

    pub(crate) fn start_range_from_pattern(&self, pat: &[u8]) -> BucketMatch {
        if self.sa.is_empty() {
            BucketMatch::NoMatch
        } else {
            get_bucket(&self.bkt, pat)
        }
    }

    pub(crate) fn start_range(&self) -> SaMatchRangeInclusive {
        if self.sa.is_empty() {
            SaMatchRangeInclusive::empty()
        } else {
            SaMatchRangeInclusive::new(0, self.sa.len() as u32 - 1)
        }
    }

    #[inline]
    pub(crate) fn refine_bounds(
        &self,
        init_range: SaMatchRangeInclusive,
        pat_sym: u8,
        offset: usize,
        text: &[u8],
    ) -> Option<SaMatchRangeInclusive> {
        if init_range.is_empty() {
            return None;
        }
        let sa_range = &self.sa[&init_range];

        // refine left bound
        let mut new_left = init_range.start;
        new_left += sa_range.partition_point(|&probe| {
            // we might be going past the end of the text with the probe + offset
            if let Some(text_sym) = text.get(probe as usize + offset) {
                return *text_sym < pat_sym;
            }
            true
        }) as u32;

        // refine right bound
        let mut new_right = init_range.start;
        new_right += sa_range.partition_point(|&probe| {
            // we might be going past the end of the text with the probe + offset
            if let Some(text_sym) = text.get(probe as usize + offset) {
                return *text_sym <= pat_sym;
            }
            true
        }) as u32;

        // there must be a match if those two are different
        if new_left < new_right {
            if let Some(chr) = text.get(self.sa[new_left as usize] as usize + offset) {
                if *chr != pat_sym {
                    return None;
                }
            } else {
                return None;
            }
            if let Some(chr) = text.get(self.sa[new_right as usize - 1] as usize + offset) {
                if *chr != pat_sym {
                    return None;
                }
            } else {
                return None;
            }
            Some(SaMatchRangeInclusive::new(new_left, new_right - 1))
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    macro_rules! sa_range {
        [$s:tt..=$e:tt] => {
            SaMatchRangeInclusive::new($s, $e)
        };
        [$s:tt..$e:tt] => {
            SaMatchRangeInclusive::new($s, $e-1)
        };
    }

    #[test]
    fn banana_saca() {
        let text = "banana$";
        let mut sa = vec![0u32; text.len()];
        saca(text.as_bytes(), &mut sa);

        //         0123456
        // text => banana$
        // sa => $ (6), a$ (5), ana$ (3), anana$ (1), banana$ (0), na$ (4), nana$ (2)
        assert_eq!(sa, vec![6, 5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn banana_unigram_buckets() {
        let text = "banana$";
        // sa == [6, 5, 3, 1, 0, 4, 2]
        //        $  a  a  a  b  n  n
        //           $  n  n
        //        0  1  2  3  4  5  6
        let buckets = compute_buckets(text.as_bytes());
        assert_eq!(
            get_bucket(&buckets, "$".as_bytes()),
            BucketMatch::Match {
                num_matched: 1,
                range: sa_range![0u32..1u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "a".as_bytes()),
            BucketMatch::Match {
                num_matched: 1,
                range: sa_range![1u32..4u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "an".as_bytes()),
            BucketMatch::Match {
                num_matched: 2,
                range: sa_range![2u32..4u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "".as_bytes()),
            BucketMatch::Match {
                num_matched: 0,
                range: sa_range![0u32..7u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "n".as_bytes()),
            BucketMatch::Match {
                num_matched: 1,
                range: sa_range![5u32..7u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "na".as_bytes()),
            BucketMatch::Match {
                num_matched: 2,
                range: sa_range![5u32..7u32]
            }
        );
    }

    #[test]
    fn banana_refine() {
        let text = "banana$";
        let sa = SuffixArray::new(text.as_bytes());

        let start_range = sa.start_range();
        let refined_range = sa.refine_bounds(start_range, b'k', 0, text.as_bytes());
        assert_eq!(refined_range, None);

        let start_range = sa.start_range();
        let refined_range = sa.refine_bounds(start_range, b'a', 0, text.as_bytes());
        assert_eq!(refined_range, Some(sa_range![1..=3]));

        let refined_range = sa.refine_bounds(refined_range.unwrap(), b'n', 1, text.as_bytes());
        assert_eq!(refined_range, Some(sa_range![2..=3]));

        let refined_range = sa.refine_bounds(refined_range.unwrap(), b'a', 2, text.as_bytes());
        assert_eq!(refined_range, Some(sa_range![2..=3]));

        let refined_range = sa.refine_bounds(refined_range.unwrap(), b'n', 3, text.as_bytes());
        assert_eq!(refined_range, Some(sa_range![3..=3]));

        let refined_range = sa.refine_bounds(refined_range.unwrap(), b'g', 3, text.as_bytes());
        assert_eq!(refined_range, None);
    }

    proptest! {
        #[test]
        fn random_refine(text: String) {
            let mut byte_counts = vec![0usize;u8::MAX as usize+1];
            for b in text.as_bytes().iter() {
                byte_counts[*b as usize] += 1;
            }
            let sa = SuffixArray::new(text.as_bytes());
            let buckets = compute_buckets(text.as_bytes());
            for (chr,cnt) in byte_counts.into_iter().enumerate() {
                let start_range = sa.start_range();
                let refined_range = sa.refine_bounds(start_range, chr as u8, 0, text.as_bytes());
                let data = [chr as u8];

                if cnt == 0 {
                    let res = get_bucket(&buckets,data.as_slice());
                    assert_eq!(res,BucketMatch::NoMatch);
                    assert_eq!(refined_range,None);
                } else {
                    let res = get_bucket(&buckets,data.as_slice());
                    assert_eq!(
                        res,
                        BucketMatch::Match {
                            num_matched: 1,
                            range: refined_range.unwrap()
                        }
                    );
                }
            }
        }
    }

    prop_compose! {
        fn text_and_index()(text in ".+")
                           (index in 0..text.as_bytes().len(),text in Just(text))
                        -> (String, usize) {
           (text, index)
       }
    }

    proptest! {

        #[test]
        fn random_refine_multi_not_found(text: String,pattern: String) {
            let sa = SuffixArray::new(text.as_bytes());
            let mut start_range = sa.start_range();

            for (offset, chr) in pattern.as_bytes().iter().enumerate() {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                let mut found = false;
                for text_window in text.as_bytes().windows(offset+1) {
                    if text_window == &pattern.as_bytes()[..offset+1] {
                        found = true;
                        break;
                    }
                }
                if found {
                    assert!(refined_range.is_some());
                } else {
                    assert!(refined_range.is_none());
                    break;
                }
                start_range = refined_range.unwrap();
            }
        }


        #[test]
        fn random_refine_multi((text, index) in text_and_index()) {
            let u32_idx = index as u32;
            let sa = SuffixArray::new(text.as_bytes());
            let mut start_range = sa.start_range();
            for (offset, chr) in text.as_bytes()[index..].iter().enumerate() {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                assert!(refined_range.is_some());
                start_range = refined_range.unwrap();
                let sa_range = &sa[start_range];
                assert!(sa_range.contains(&u32_idx));
            }
        }
    }

    proptest! {
        #[test]
        fn random_refine_multi_with_buckets_not_found(text: String,pattern: String) {
            let sa = SuffixArray::new(text.as_bytes());
            let pattern = &pattern.as_bytes();
            let (mut start_range, mut offset) = match sa.start_range_from_pattern(pattern) {
                BucketMatch::NoMatch => {
                    let mut found = false;
                    let pattern_len = pattern.len().min(2);
                    for text_window in text.as_bytes().windows(pattern_len) {
                        if text_window == &pattern[..pattern_len] {
                            found = true;
                            break;
                        }
                    }
                    assert!(!found);
                    return Ok(())
                },
                BucketMatch::Match{ num_matched, range } => (range,num_matched)
            };
            for chr in pattern.iter().skip(offset) {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                let mut found = false;
                for text_window in text.as_bytes().windows(offset+1) {
                    if text_window == &pattern[..offset+1] {
                        found = true;
                        break;
                    }
                }
                if found {
                    assert!(refined_range.is_some());
                } else {
                    assert!(refined_range.is_none());
                    break;
                }
                start_range = refined_range.unwrap();
                offset += 1;
            }
        }
    }

    proptest! {
        #[test]
        fn random_refine_multi_with_buckets((text, index) in text_and_index()) {
            let u32_idx = index as u32;
            let sa = SuffixArray::new(text.as_bytes());
            let pattern = &text.as_bytes()[index..];
            let (mut start_range, mut offset) = match sa.start_range_from_pattern(pattern) {
                BucketMatch::NoMatch => panic!(),
                BucketMatch::Match{ num_matched, range } => (range,num_matched)
            };
            for chr in text.as_bytes()[index..].iter().skip(offset) {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                assert!(refined_range.is_some());
                start_range = refined_range.unwrap();
                let sa_range = &sa[start_range];
                assert!(sa_range.contains(&u32_idx));
                offset += 1;
            }
        }
    }

    proptest! {
        #[test]
        fn random_unigram_buckets(text: String) {
            let mut byte_counts = vec![0usize;u8::MAX as usize+1];
            for b in text.as_bytes().iter() {
                byte_counts[*b as usize] += 1;
            }
            let buckets = compute_buckets(text.as_bytes());
            for (chr,cnt) in byte_counts.into_iter().enumerate() {
                let data = [chr as u8];
                let res = get_bucket(&buckets,data.as_slice());
                match res {
                    BucketMatch::NoMatch => assert_eq!(cnt,0),
                    BucketMatch::Match { num_matched: 1, range} => assert_eq!(range.len(),cnt),
                    _ => panic!()
                }
            }
        }
    }
}
