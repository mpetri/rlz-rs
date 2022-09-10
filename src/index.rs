use crate::factor::FactorType;
use crate::{config, dict};
mod suffix_array;

use suffix_array::SuffixArray;

use self::suffix_array::{SuffixArrayMatch, SuffixArrayRangeInclusive};

pub(crate) struct Index {
    text: Vec<u8>,
    sa: suffix_array::SuffixArray,
    config: config::Compression,
}

enum IndexSearchResult {
    NoMatch,
    Match { num_matched: u32, offset: u32 },
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
        bounds: SuffixArrayRangeInclusive,
        pat_sym: u8,
        offset: usize,
    ) -> SuffixArrayRangeInclusive {
        self.sa.refine_bounds(bounds, pat_sym, offset, &self.text)
    }

    fn find_longest_match(&self, pattern: &[u8]) -> IndexSearchResult {
        let (mut bounds, mut num_matched) = match self.sa.start_range_from_pattern(pattern) {
            SuffixArrayMatch::NoMatch => return IndexSearchResult::NoMatch,
            SuffixArrayMatch::Match { num_matched, range } => (range, num_matched),
        };

        while let Some(next_sym) = pattern.get(num_matched) {
            match self.refine_bounds(bounds, *next_sym, num_matched) {
                SuffixArrayRangeInclusive::Empty => break,
                other => {
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
            SuffixArrayRangeInclusive::Range { start, end } => {
                // we match! take it as far as possible
                let text_pos = self.sa[start as usize] as usize;
                while let Some(next_sym) = pattern.get(num_matched) {
                    if let Some(text_sym) = self.text.get(text_pos + num_matched) {
                        if next_sym == text_sym {
                            num_matched += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                text_pos as u32
            }
        };
        IndexSearchResult::Match {
            num_matched: num_matched as u32,
            offset,
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
        let longest_match = self.index.find_longest_match(self.remaining_input);
        let found_factor = match longest_match {
            IndexSearchResult::NoMatch => FactorType::Literal(&self.remaining_input[0..1]),
            IndexSearchResult::Match {
                num_matched,
                offset,
            } => {
                if num_matched <= self.config.literal_threshold {
                    FactorType::Literal(&self.remaining_input[0..num_matched as usize])
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
        self.remaining_input = &self.remaining_input[found_factor.len()..];

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
        let config = crate::Compression {
            literal_threshold: 1,
            ..Default::default()
        };
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
