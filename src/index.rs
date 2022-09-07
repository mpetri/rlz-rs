use std::ops::{Range, RangeInclusive};

use crate::factor::{self, FactorType};
use crate::{config, dict};
mod suffix_array;

use suffix_array::SuffixArray;

use self::suffix_array::SaMatchRangeInclusive;

pub(crate) struct Index {
    text: Vec<u8>,
    sa: suffix_array::SuffixArray,
    config: config::Compression,
}

enum MatchType {
    Matched(SaMatchRangeInclusive),
    NotMatched,
}

impl Index {
    pub(crate) fn from_dict(text: Vec<u8>, config: &config::Compression) -> Self {
        let sa = SuffixArray::new(&text);
        Self {
            text,
            sa,
            config: config.clone(),
        }
    }

    pub(crate) fn factorize<'encoder, 'input>(
        &'encoder self,
        input: &'input [u8],
    ) -> FactorIterator<'encoder, 'input> {
        FactorIterator {
            index: self,
            remaining_input: input,
            config: &self.config,
        }
    }

    fn refine_bounds(
        &self,
        bounds: SaMatchRangeInclusive,
        pat_sym: u8,
        offset: usize,
    ) -> MatchType {
        match self.sa.refine_bounds(bounds, pat_sym, offset, &self.text) {
            Some(new_bounds) => MatchType::Matched(new_bounds),
            None => MatchType::NotMatched,
        }
    }
}

pub(crate) struct FactorIterator<'encoder, 'input> {
    index: &'encoder Index,
    remaining_input: &'input [u8],
    config: &'encoder config::Compression,
}

impl<'encoder, 'input> Iterator for FactorIterator<'encoder, 'input> {
    type Item = FactorType<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_input.is_empty() {
            return None;
        }

        let mut bounds = self.index.sa.start_range();
        let mut num_matched = 0;
        // let (mut bounds, mut num_matched) =
        //     self.index.sa.start_range_from_pattern(self.remaining_input);
        /* refine bounds as long as possible */
        while let Some(next_sym) = self.remaining_input.get(num_matched) {
            match self.index.refine_bounds(bounds, *next_sym, num_matched) {
                MatchType::Matched(new_bounds) => {
                    num_matched += 1;
                    bounds = new_bounds;
                    if bounds.len() == 1 {
                        break;
                    }
                }
                MatchType::NotMatched => break,
            }
        }

        // if we have single match take it as far as possible
        if num_matched != 0 && bounds.len() == 1 {
            let text_pos = self.index.sa[bounds.start as usize] as usize;
            while let Some(next_sym) = self.remaining_input.get(num_matched) {
                if let Some(text_sym) = self.index.text.get(text_pos + num_matched) {
                    if next_sym == text_sym {
                        num_matched += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let found_factor = if num_matched <= self.config.literal_threshold {
            // we always match at least one
            if num_matched == 0 {
                num_matched += 1;
            }
            FactorType::Literal(&self.remaining_input[0..num_matched])
        } else {
            let num_matched = num_matched as u32;
            match self.config.factor_selection {
                factor::Selection::First => FactorType::Copy {
                    offset: self.index.sa[bounds.start as usize],
                    len: num_matched,
                },
                factor::Selection::Last => FactorType::Copy {
                    offset: self.index.sa[bounds.end as usize - 1],
                    len: num_matched,
                },
            }
        };

        // advance text pointers
        self.remaining_input = &self.remaining_input[num_matched..];

        Some(found_factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn banana_factorize() {
        let text = "banana$";
        let mut config = crate::Compression::default();
        config.literal_threshold = 1;
        let index = Index::from_dict(text.as_bytes().to_vec(), &config);

        let input = "bac$anana";

        let mut factors = index.factorize(input.as_bytes());
        let first = factors.next();
        assert_eq!(first, Some(FactorType::Copy { offset: 0, len: 2 }));
        let second = factors.next();
        assert_eq!(second, Some(FactorType::Literal(&[b'c'])));
        let third = factors.next();
        assert_eq!(third, Some(FactorType::Literal(&[b'$'])));
        let forth = factors.next();
        assert_eq!(forth, Some(FactorType::Copy { offset: 1, len: 5 }));
        assert_eq!(factors.next(), None);
    }
}
