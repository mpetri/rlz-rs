use std::ops::Range;

use crate::factor::{self, FactorType};
use crate::{config, dict};
mod suffix_array;

use suffix_array::SuffixArray;

pub(crate) struct Index {
    dict: dict::Dictionary,
    sa: suffix_array::SuffixArray,
    config: config::Compression,
}

enum MatchType {
    Matched(Range<usize>),
    NotMatched,
}

impl Index {
    pub(crate) fn from_dict(dict: dict::Dictionary, config: &config::Compression) -> Self {
        let sa = SuffixArray::new(&dict);
        Self {
            dict,
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
            remaining_input: &input,
            config: &self.config,
        }
    }

    pub(crate) fn refine_bounds(&self, bounds: Range<usize>, sym: u8, offset: usize) -> MatchType {}
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

        let mut bounds = 0..self.index.sa.len() - 1;
        let mut num_matched = 0;
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
            let text_pos = self.index.sa[bounds.start] as usize;
            while let Some(next_sym) = self.remaining_input.get(num_matched) {
                if let Some(text_sym) = self.index.dict.get(text_pos + num_matched) {
                    if next_sym == text_sym {
                        num_matched += 1;
                    } else {
                        break;
                    }
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
                    offset: self.index.sa[bounds.start],
                    len: num_matched,
                },
                factor::Selection::Last => FactorType::Copy {
                    offset: self.index.sa[bounds.end - 1],
                    len: num_matched,
                },
            }
        };

        // advance text pointers
        self.remaining_input = &self.remaining_input[num_matched..];

        Some(found_factor)
    }
}
