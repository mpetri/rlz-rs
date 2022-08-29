use crate::{coder, factor};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Compression {
    pub factor_selection: factor::Selection,
    pub literal_threshold: usize,
    pub local_search: factor::LocalSearch,
    pub factor_compression: coder::Coder,
}

impl Compression {
    fn new() -> Compression {
        Compression {
            factor_selection: factor::Selection::default(),
            literal_threshold: 3,
            local_search: factor::LocalSearch::default(),
            factor_compression: coder::Coder::default(),
        }
    }
}

impl Default for Compression {
    fn default() -> Compression {
        Compression::new()
    }
}
