#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Selection {
    First,
    Last,
}

impl Default for Selection {
    fn default() -> Selection {
        Selection::First
    }
}

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

pub(crate) enum FactorType<'input> {
    Literal(&'input [u8]),
    Copy { offset: u32, len: u32 },
}
