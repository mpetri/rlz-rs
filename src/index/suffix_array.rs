// mostly taken from the suffix_array crate but with modifications

use serde::{Deserialize, Serialize};
use std::{ops::Deref, slice::from_raw_parts_mut};

use cdivsufsort::sort_in_place as divsufsort;
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SuffixArrayRangeInclusive {
    Empty,
    Range { start: u32, end: u32 },
}

impl SuffixArrayRangeInclusive {
    pub(crate) fn is_singleton(&self) -> bool {
        match self {
            SuffixArrayRangeInclusive::Empty => false,
            SuffixArrayRangeInclusive::Range { start, end } => start == end,
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> u32 {
        match self {
            SuffixArrayRangeInclusive::Empty => 0,
            SuffixArrayRangeInclusive::Range { start, end } => *end - *start + 1,
        }
    }
}

macro_rules! sa_range {
    [$s:tt..=$e:tt] => {
        SuffixArrayRangeInclusive::Range{start: $s, end:$e}
    };
    [$s:tt..$e:tt] => {
        SuffixArrayRangeInclusive::Range{start: $s, end: $e as u32 -1}
    };
}

impl std::ops::Index<SuffixArrayRangeInclusive> for Vec<u32> {
    type Output = [u32];

    fn index(&self, range: SuffixArrayRangeInclusive) -> &Self::Output {
        match range {
            SuffixArrayRangeInclusive::Empty => &[],
            SuffixArrayRangeInclusive::Range { start, end } => &self[start as usize..=end as usize],
        }
    }
}

impl std::ops::Index<&SuffixArrayRangeInclusive> for Vec<u32> {
    type Output = [u32];

    fn index(&self, range: &SuffixArrayRangeInclusive) -> &Self::Output {
        match range {
            SuffixArrayRangeInclusive::Empty => &[],
            SuffixArrayRangeInclusive::Range { start, end } => {
                &self[*start as usize..=*end as usize]
            }
        }
    }
}

/// Maximum length of the input string.
pub const MAX_LENGTH: usize = std::i32::MAX as usize;

/// Wrapper of the underlying suffix array construction algorithm.
#[tracing::instrument]
pub fn saca(s: &[u8], sa: &mut [u32]) {
    assert!(s.len() <= MAX_LENGTH);
    divsufsort(s, as_signed_integer_slice(&mut sa[..]));
}

fn as_signed_integer_slice(sa: &mut [u32]) -> &mut [i32] {
    unsafe {
        let len = sa.len();
        let data = sa.as_mut_ptr().cast::<i32>();
        from_raw_parts_mut(data, len)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SuffixArray {
    sa: Vec<u32>,
    bkt: Vec<SuffixArrayRangeInclusive>,
}

#[tracing::instrument]
fn compute_buckets(text: &[u8]) -> Vec<SuffixArrayRangeInclusive> {
    let num_uniq_chars: usize = u8::MAX as usize + 1;
    let num_zero_grams = 1;
    let num_bigrams = num_uniq_chars * num_uniq_chars;
    let num_buckets: usize = num_bigrams + num_uniq_chars + num_zero_grams;
    let mut bkt_cnts = vec![0; num_buckets];
    let mut bkt = vec![SuffixArrayRangeInclusive::Empty; num_buckets];

    if text.is_empty() {
        return bkt;
    }

    // full range
    let tlen = text.len() as u32;
    bkt[0] = sa_range![0..tlen];

    // count occurrences.
    for bigram in text.windows(2) {
        let c0 = unsafe { *bigram.get_unchecked(0) };
        let c1 = unsafe { *bigram.get_unchecked(1) };
        let bigram_idx = c0 as usize * num_uniq_chars + c1 as usize;
        let bigram_idx = bigram_idx + num_uniq_chars + num_zero_grams;
        bkt_cnts[bigram_idx] += 1;
        bkt_cnts[c0 as usize + num_zero_grams] += 1;
    }
    // window(2) misses the last sym
    let last_sym = *text.last().unwrap() as usize;
    bkt_cnts[last_sym + num_zero_grams] += 1;

    // fill unigrams first
    let mut sum = 0;
    for uidx in 0..num_uniq_chars {
        let uidx = uidx + num_zero_grams;
        if bkt_cnts[uidx] != 0 {
            let start = sum;
            let end = sum + bkt_cnts[uidx] as u32;
            bkt[uidx] = sa_range![start..end];
            sum += bkt_cnts[uidx];
            bkt_cnts[uidx] = start;
        }
    }

    // fill bigrams
    for first in 0..num_uniq_chars {
        let uidx = first + num_zero_grams;
        let mut sum = bkt_cnts[uidx];
        if first == last_sym {
            sum += 1;
        }
        for second in 0..num_uniq_chars {
            let bigram_idx = first * num_uniq_chars + second as usize;
            let bigram_idx = bigram_idx + num_uniq_chars + num_zero_grams;
            if bkt_cnts[bigram_idx] != 0 {
                let start = sum;
                let end = sum + bkt_cnts[bigram_idx] as u32;
                bkt[bigram_idx] = sa_range![start..end];
                sum += bkt_cnts[bigram_idx];
            }
        }
    }

    bkt
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]

pub(crate) enum SuffixArrayMatch {
    NoMatch,
    Match {
        num_matched: usize,
        range: SuffixArrayRangeInclusive,
    },
}

#[tracing::instrument]
fn get_bucket(bkt: &[SuffixArrayRangeInclusive], pat: &[u8]) -> SuffixArrayMatch {
    let num_uniq_chars: usize = u8::MAX as usize + 1;
    let num_zero_grams = 1;
    match pat.len() {
        0 => SuffixArrayMatch::Match {
            num_matched: 0,
            range: bkt[0],
        },
        1 => {
            // top-level bucket (c0, $)..=(c0, 255).
            let c0 = pat[0] as usize + num_zero_grams;
            if bkt[c0] == SuffixArrayRangeInclusive::Empty {
                SuffixArrayMatch::NoMatch
            } else {
                SuffixArrayMatch::Match {
                    num_matched: 1,
                    range: bkt[c0],
                }
            }
        }
        _ => {
            // sub-bucket (c0, c1).
            let first = pat[0];
            let second = pat[1];
            let bigram_idx = first as usize * num_uniq_chars + second as usize;
            let bigram_idx = bigram_idx + num_uniq_chars + num_zero_grams;
            if bkt[bigram_idx] == SuffixArrayRangeInclusive::Empty {
                // bi-gram is empty, return unigram
                return get_bucket(bkt, &pat[..1]);
            }
            SuffixArrayMatch::Match {
                num_matched: 2,
                range: bkt[bigram_idx],
            }
        }
    }
}

impl SuffixArray {
    #[tracing::instrument]
    pub fn new(text: &[u8]) -> Self {
        let mut sa = vec![0; text.len()];
        saca(text, &mut sa[..]);
        SuffixArray {
            bkt: compute_buckets(text),
            sa,
        }
    }

    pub(crate) fn start_range_from_pattern(&self, pat: &[u8]) -> SuffixArrayMatch {
        get_bucket(&self.bkt, pat)
    }

    #[cfg(test)]
    pub(crate) fn start_range(&self) -> SuffixArrayRangeInclusive {
        self.bkt[0]
    }

    #[inline]
    pub(crate) fn refine_bounds(
        &self,
        init_range: SuffixArrayRangeInclusive,
        pat_sym: u8,
        offset: usize,
        text: &[u8],
    ) -> SuffixArrayRangeInclusive {
        let (mut new_left, mut new_right) = match init_range {
            SuffixArrayRangeInclusive::Empty => return init_range,
            SuffixArrayRangeInclusive::Range { start, end: _ } => (start, start),
        };
        let sa_range = &self.sa[&init_range];

        // refine left bound
        new_left += sa_range.partition_point(|&probe| {
            // we might be going past the end of the text with the probe + offset
            if let Some(text_sym) = text.get(probe as usize + offset) {
                return *text_sym < pat_sym;
            }
            true
        }) as u32;

        // refine right bound
        new_right += sa_range.partition_point(|&probe| {
            // we might be going past the end of the text with the probe + offset
            if let Some(text_sym) = text.get(probe as usize + offset) {
                return *text_sym <= pat_sym;
            }
            true
        }) as u32;

        // there must be a match if those two are different
        if new_left < new_right {
            sa_range![new_left..new_right]
        } else {
            SuffixArrayRangeInclusive::Empty
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
    fn banana_buckets() {
        let text = "banana$";
        // sa == [6, 5, 3, 1, 0, 4, 2]
        //        $  a  a  a  b  n  n
        //           $  n  n
        //        0  1  2  3  4  5  6
        let buckets = compute_buckets(text.as_bytes());
        assert_eq!(
            get_bucket(&buckets, "$".as_bytes()),
            SuffixArrayMatch::Match {
                num_matched: 1,
                range: sa_range![0u32..1u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "a".as_bytes()),
            SuffixArrayMatch::Match {
                num_matched: 1,
                range: sa_range![1u32..4u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "an".as_bytes()),
            SuffixArrayMatch::Match {
                num_matched: 2,
                range: sa_range![2u32..4u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "".as_bytes()),
            SuffixArrayMatch::Match {
                num_matched: 0,
                range: sa_range![0u32..7u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "n".as_bytes()),
            SuffixArrayMatch::Match {
                num_matched: 1,
                range: sa_range![5u32..7u32]
            }
        );
        assert_eq!(
            get_bucket(&buckets, "na".as_bytes()),
            SuffixArrayMatch::Match {
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
        assert_eq!(refined_range, SuffixArrayRangeInclusive::Empty);

        let start_range = sa.start_range();
        let refined_range = sa.refine_bounds(start_range, b'a', 0, text.as_bytes());
        assert_eq!(refined_range, sa_range![1..=3]);

        let refined_range = sa.refine_bounds(refined_range, b'n', 1, text.as_bytes());
        assert_eq!(refined_range, sa_range![2..=3]);

        let refined_range = sa.refine_bounds(refined_range, b'a', 2, text.as_bytes());
        assert_eq!(refined_range, sa_range![2..=3]);

        let refined_range = sa.refine_bounds(refined_range, b'n', 3, text.as_bytes());
        assert_eq!(refined_range, sa_range![3..=3]);

        let refined_range = sa.refine_bounds(refined_range, b'g', 3, text.as_bytes());
        assert_eq!(refined_range, SuffixArrayRangeInclusive::Empty);
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
                    assert_eq!(res,SuffixArrayMatch::NoMatch);
                    assert_eq!(refined_range,SuffixArrayRangeInclusive::Empty);
                } else {
                    let res = get_bucket(&buckets,data.as_slice());
                    assert_eq!(
                        res,
                        SuffixArrayMatch::Match {
                            num_matched: 1,
                            range: refined_range
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
                    assert_ne!(refined_range,SuffixArrayRangeInclusive::Empty);
                } else {
                    assert_eq!(refined_range,SuffixArrayRangeInclusive::Empty);
                    break;
                }
                start_range = refined_range;
            }
        }


        #[test]
        fn random_refine_multi((text, index) in text_and_index()) {
            let u32_idx = index as u32;
            let sa = SuffixArray::new(text.as_bytes());
            let mut start_range = sa.start_range();
            for (offset, chr) in text.as_bytes()[index..].iter().enumerate() {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                assert_ne!(refined_range,SuffixArrayRangeInclusive::Empty);
                start_range = refined_range;
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
                SuffixArrayMatch::NoMatch => {
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
                SuffixArrayMatch::Match{ num_matched, range } => (range,num_matched)
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
                    assert_ne!(refined_range,SuffixArrayRangeInclusive::Empty);
                } else {
                    assert_eq!(refined_range,SuffixArrayRangeInclusive::Empty);
                    break;
                }
                start_range = refined_range;
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
                SuffixArrayMatch::NoMatch => panic!(),
                SuffixArrayMatch::Match{ num_matched, range } => (range, num_matched)
            };
            if pattern.len() >= 2 {
                assert_eq!(offset,2);
            }
            for chr in text.as_bytes()[index..].iter().skip(offset) {
                let refined_range = sa.refine_bounds(start_range, *chr, offset, text.as_bytes());
                dbg!(&refined_range);
                assert_ne!(refined_range,SuffixArrayRangeInclusive::Empty);
                start_range = refined_range;
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
                    SuffixArrayMatch::NoMatch => assert_eq!(cnt,0),
                    SuffixArrayMatch::Match { num_matched: 1, range} => assert_eq!(range.len() as usize,cnt),
                    _ => panic!()
                }
            }
        }
    }
}
