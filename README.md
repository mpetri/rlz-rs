# Relative-Lempel Ziv (RLZ) [![Crates.io][crates-badge]][crates-url] [![Docs.rs][docs-badge]][docs-rs] [![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/rbz.svg
[crates-url]: https://crates.io/crates/rbz
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://opensource.org/licenses/MIT
[docs-rs]: https://docs.rs/rbz
[docs-badge]: https://img.shields.io/docsrs/rbz/0.1.0

A Relative-Lempel Ziv (RLZ) based LZ compressor that compresses against a large static dictionary.

This code implements the RLZ compressor, as described in:

```bibtex
@article{DBLP:journals/pvldb/HoobinPZ11,
  author    = {Christopher Hoobin and
               Simon J. Puglisi and
               Justin Zobel},
  title     = {Relative Lempel-Ziv Factorization for Efficient Storage and Retrieval
               of Web Collections},
  journal   = {Proc. {VLDB} Endow.},
  volume    = {5},
  number    = {3},
  pages     = {265--273},
  year      = {2011},
}
```

```bibtex
@article{DBLP:journals/corr/PetriMNW16,
  author    = {Matthias Petri and
               Alistair Moffat and
               P. C. Nagesh and
               Anthony Wirth},
  title     = {Access Time Tradeoffs in Archive Compression},
  journal   = {CoRR},
  volume    = {abs/1602.08829},
  year      = {2016},
  url       = {http://arxiv.org/abs/1602.08829},
  eprinttype = {arXiv},
}
```

```bibtex
@inproceedings{DBLP:conf/www/LiaoPMW16,
  author    = {Kewen Liao and
               Matthias Petri and
               Alistair Moffat and
               Anthony Wirth},
  title     = {Effective Construction of Relative Lempel-Ziv Dictionaries},
  booktitle = {Proceedings of the 25th International Conference on World Wide Web,
               {WWW} 2016, Montreal, Canada, April 11 - 15, 2016},
}
```


# What is RLZ (taken from the paper)

The Relative Lempel-Ziv (RLZ) scheme is a hybrid of several phrase-based compression mechanisms. Encoding is based on a fixed-text dictionary, with all substrings within the dictionary available for use as factors, in the style of LZ77. But the dictionary is constructed in a semi-static manner, and hence needs to be representative of the entire text being coded if compression effectiveness is not to be compromised. Furthermore, because
RLZ is intended for large web-based archives when constructing the dictionary, it is infeasible to have the whole input text in memory.

# Usage


```rust
use rlz::RlzCompressor;

let dict = Dictionary::from(b"banana");

let rlz_compressor = RlzCompressor::builder().build_from_dict(dict);

let mut output = Vec::new();

let encoded_len = rlz_compressor.encode(&text[..],&mut output)?;
assert_eq!(encoded_len,output.len());

let mut stored_decoder = Vec::new();
rlz_compressor.store(&mut stored_decoder)?;

let loaded_decoder = RlzCompressor::load(&stored_decoder[..])?;

let mut recovered = Vec::new();
loaded_decoder.decode(&output[..],&mut recovered)?;

assert_eq!(recovered,text);
```

# License

MIT
