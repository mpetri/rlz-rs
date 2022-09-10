#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LocalSearch {
    None,
    Window(usize),
}

impl Default for LocalSearch {
    fn default() -> LocalSearch {
        LocalSearch::None
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]

pub(crate) enum FactorType<'input> {
    Literal(&'input [u8]),
    Copy { offset: u32, len: u32 },
}

impl<'input> FactorType<'_> {
    pub(crate) fn len(&self) -> usize {
        match &self {
            FactorType::Literal(lit) => lit.len(),
            FactorType::Copy { offset, len } => *len as usize,
        }
    }
}
