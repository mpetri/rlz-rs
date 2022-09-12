use crate::factor::FactorType;
use crate::{config, dict, Dictionary};
mod suffix_array;

use bytes::Buf;
use suffix_array::SuffixArray;

use self::suffix_array::{SuffixArrayMatch, SuffixArrayRangeInclusive};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Index {
    sa: suffix_array::SuffixArray,
    pub(crate) config: config::Configuration,
}

enum IndexSearchResult {
    NoMatch,
    Match { num_matched: u32, offset: u32 },
}

impl Index {
    pub(crate) fn from_dict(dict: &Dictionary, config: &config::Configuration) -> Self {
        let sa = SuffixArray::new(dict);
        Self {
            sa,
            config: config.clone(),
        }
    }

    pub(crate) fn factorize<'dict>(
        &'_ self,
        dict: &'dict dict::Dictionary,
        mut input: impl Buf,
    ) -> FactorIterator<'dict, '_> {
        FactorIterator {
            dict,
            index: self,
            remaining_input: input.copy_to_bytes(input.remaining()),
            config: &self.config,
        }
    }

    fn refine_bounds(
        &self,
        dict: &dict::Dictionary,
        bounds: SuffixArrayRangeInclusive,
        pat_sym: u8,
        offset: usize,
    ) -> SuffixArrayRangeInclusive {
        self.sa.refine_bounds(bounds, pat_sym, offset, dict)
    }

    #[tracing::instrument(skip_all)]
    fn find_longest_match(&self, dict: &dict::Dictionary, pattern: &[u8]) -> IndexSearchResult {
        let (mut bounds, mut num_matched) = match self.sa.start_range_from_pattern(pattern) {
            SuffixArrayMatch::NoMatch => return IndexSearchResult::NoMatch,
            SuffixArrayMatch::Match { num_matched, range } => (range, num_matched),
        };

        while let Some(next_sym) = pattern.get(num_matched) {
            match self.refine_bounds(dict, bounds, *next_sym, num_matched) {
                SuffixArrayRangeInclusive::Empty => break,
                other @ SuffixArrayRangeInclusive::Range { .. } => {
                    num_matched += 1;
                    bounds = other;
                    if other.is_singleton() {
                        break;
                    }
                }
            }
        }

        let offset = match bounds {
            SuffixArrayRangeInclusive::Empty => {
                panic!("this should never happen at this point because we have at least one match")
            }
            SuffixArrayRangeInclusive::Range { start, end: _ } => {
                // we match! take it as far as possible
                let text_pos = self.sa[start as usize];
                while let Some(next_sym) = pattern.get(num_matched) {
                    if let Some(text_sym) = dict.get(text_pos as usize + num_matched) {
                        if next_sym == text_sym {
                            num_matched += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                text_pos
            }
        };
        IndexSearchResult::Match {
            num_matched: num_matched as u32,
            offset,
        }
    }
}

pub(crate) struct FactorIterator<'dict, 'encoder> {
    dict: &'dict dict::Dictionary,
    index: &'encoder Index,
    remaining_input: bytes::Bytes,
    config: &'encoder config::Configuration,
}

impl<'dict, 'encoder> Iterator for FactorIterator<'dict, 'encoder> {
    type Item = FactorType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_input.is_empty() {
            return None;
        }
        let longest_match = self
            .index
            .find_longest_match(self.dict, &self.remaining_input);
        let found_factor = match longest_match {
            IndexSearchResult::NoMatch => FactorType::Literal(self.remaining_input.slice(0..1)),
            IndexSearchResult::Match {
                num_matched,
                offset,
            } => {
                if num_matched <= self.config.literal_threshold {
                    FactorType::Literal(self.remaining_input.slice(0..num_matched as usize))
                } else {
                    let num_matched = num_matched as u32;
                    FactorType::Copy {
                        offset,
                        len: num_matched,
                    }
                }
            }
        };

        // advance text pointers
        self.remaining_input = self.remaining_input.slice(found_factor.len()..);

        Some(found_factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banana_factorize() {
        let text = "banana$";
        let config = crate::Configuration {
            literal_threshold: 1,
            ..Default::default()
        };
        let dict = Dictionary::from(text.as_bytes());
        let index = Index::from_dict(&dict, &config);

        let input = "bac$anana";

        let mut factors = index.factorize(&dict, input.as_bytes());
        let first = factors.next();
        assert_eq!(first, Some(FactorType::Copy { offset: 0, len: 2 }));
        let second = factors.next();
        assert_eq!(
            second,
            Some(FactorType::Literal(bytes::Bytes::from_static(&[b'c'])))
        );
        let third = factors.next();
        assert_eq!(
            third,
            Some(FactorType::Literal(bytes::Bytes::from_static(&[b'$'])))
        );
        let forth = factors.next();
        assert_eq!(forth, Some(FactorType::Copy { offset: 1, len: 5 }));
        assert_eq!(factors.next(), None);
    }
}
