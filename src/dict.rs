use std::ops::Deref;

pub struct Dictionary(Vec<u8>);

impl Dictionary {
    pub fn builder(dict_mib: usize, reservoir_mib: usize) -> DictionaryBuilder {
        DictionaryBuilder::default()
    }
}

#[derive(Default)]
pub struct DictionaryBuilder {
    dict_size: usize,
    //reservoir: Reservoir,
}

impl DictionaryBuilder {
    pub fn sample(&mut self, bytes: &[u8]) {
        // Set the name on the builder itself, and return the builder by value.
        //self.reservoir.maybe_add(bytes);
    }

    pub fn finish(self) -> Dictionary {
        Dictionary(Vec::new())
        // Dictionary(self.reservoir.extract(self.dict_size))
    }
}

impl Deref for Dictionary {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
