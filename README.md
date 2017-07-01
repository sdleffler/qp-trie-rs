[![Build Status](https://travis-ci.org/sdleffler/qp-trie-rs.svg?branch=master)](https://travis-ci.org/sdleffler/qp-trie-rs)
[![Docs Status](https://docs.rs/qp-trie/badge.svg)](https://docs.rs/qp-trie)
[![On crates.io](https://img.shields.io/crates/v/qp-trie.svg)](https://crates.io/crates/qp-trie)

# qp-trie-rs: A QP-trie implementation in pure Rust

A QP-trie ("Quelques-bits Popcount trie" or "Quad-bit Popcount trie") is a
radix trie for keys which can be interpreted as a string of nybbles (where a
nybble is half a byte, or four bits.) QP-tries are essentially Patricia tries
which branch on nybbles instead of individual bits; as such, a QP-trie has a
branching factor (and radix) of 16.

## When should I use a QP-trie?

QP-tries as implemented in this crate are key-value maps for any keys which
implement `Borrow<[u8]>`. They are useful whenever you might need the same
operations as a `HashMap` or `BTreeMap`, but need either a bit more speed
(QP-tries are as fast or a bit faster as Rust's `HashMap` with the default
hasher) and/or the ability to efficiently query for sets of elements with a
given prefix.

QP-tries support efficient lookup/insertion/removal of individual elements,
lookup/removal of sets of values with keys with a given prefix.

## Examples

Keys can be any type which implements `Borrow<[u8]>`. Unfortunately at the
moment, this rules out `String` - while this trie can still be used to store
strings, it is necessary to manually convert them to byte slices and `Vec<u8>`s
for use as keys. Here's a naive, simple example of putting 9 2-element byte arrays
into the trie, and then removing all byte arrays which begin with "1":

```rust
use qp_trie::Trie;

let mut trie = Trie::new();

for i in 0u8..3 {
    for j in 0u8..3 {
        trie.insert([i, j], i + j);
    }
}

for i in 0u8..3 {
    trie.remove([1, i]);
}

assert!(trie.iter().all(|(&key, _)| key[0] != 1));
```

Here's a slightly less naive method, which is actually vastly more efficient:

```rust
use qp_trie::Trie;

let mut trie = Trie::new();

for i in 0u8..3 {
    trie.extend((0u8..3).map(|j| ([i, j], i + j)));
}

trie.remove_prefix([1]);

assert!(trie.iter().all(|(&key, _)| key[0] != 1));
```

Although the `extend` bit really isn't any more efficient (it's difficult to
preallocate space for `n` elements in a trie) we're guaranteed that
`trie.remove_prefix([1])` only actually removes a single node in the trie - the
parent node of all nodes with the given prefix. QP-tries, like all radix tries,
are extremely efficient when dealing with anything related to prefixes. This
extends to iteration over prefixes:

```rust
use qp_trie::Trie;

let mut trie = Trie::new();

for i in 0u8..3 {
    trie.extend((0u8..3).map(|k| ([i, j], i + j)));
}

let mut iter = trie.iter_prefix([1]);

assert_eq!(iter.next(), Some((&[1, 0], &1)));
assert_eq!(iter.next(), Some((&[1, 1], &2)));
assert_eq!(iter.next(), Some((&[1, 2], &3)));
assert_eq!(iter.next(), None);
```

## Differences from the qptrie crate

This crate originally started as a fork of the `qptrie` crate; however, I found
the code difficult to work with and full of unsafe pointer manipulation which I
felt could be avoided. To avoid a pull request which would essentially rewrite
the entire library I decided to write my own instead.

Several notable idiomatic features are provided which were missing from the `qptrie` crate:
- `.iter()` and `.iter_mut()` for immutable and mutable iteration over the key/value pairs of the trie
- `qp_trie::Trie` implements `Extend` and `IntoIterator`
- `qp_trie::Trie` implements `Index` and `IndexMut`
- `qp_trie::Trie` provides an "Entry API" with type signatures almost identical
  to that provided by the `std::collections` `BTreeMap` and `HashMap`.

In addition to being written using safer code (failures which would otherwise
cause undefined behavior will cause panics when compiled with debug assertions
enabled) `qp_trie::Trie` is slightly faster than `qptrie::Trie` according to
benchmarks based on those shown in the `qptrie` repository.

## Benchmarks

Benchmarks are run against the `qptrie` crate and the Rust stdlib `BTreeMap`
and `HashMap`. `qp_trie::Trie` consistently outperforms the `std::collections`
`BTreeMap` and `HashMap` and also the `qptrie` crate's implementation on my
machine - a Chromebook Pixel 2.0 running GalliumOS.

Benchmarks can be reproduced using `cargo bench`. The Rust version used was
`rustc 1.19.0-nightly (cfb5debbc 2017-06-12)`. Run several times, the
benchmarks are consistent in their outputs but I selected the lowest variance
results to display here.

Benchmarks named `exotrie` are using the `qptrie::Trie` implementation.

```
running 8 tests
test bench_btreemap_get    ... bench: 114,172,574 ns/iter (+/- 10,890,962)
test bench_btreemap_insert ... bench: 118,547,331 ns/iter (+/- 13,464,035)
test bench_exotrie_get     ... bench:  54,297,605 ns/iter (+/- 4,392,593)
test bench_exotrie_insert  ... bench:  62,537,678 ns/iter (+/- 21,724,153)
test bench_hashmap_get     ... bench:  63,191,541 ns/iter (+/- 6,685,288)
test bench_hashmap_insert  ... bench:  55,076,618 ns/iter (+/- 2,212,986)
test bench_trie_get        ... bench:  48,232,553 ns/iter (+/- 6,583,801)
test bench_trie_insert     ... bench:  57,935,037 ns/iter (+/- 16,538,104)

test result: ok. 0 passed; 0 failed; 0 ignored; 8 measured; 0 filtered out
```

## Future work

- Benchmark against `FxHasher`/`FnvHasher` to get a better idea of how `Trie` compares against `HashMap`.
- Add wrapper types for `String` and `str` to make working with strings easier.

## License

The `qp-trie-rs` crate is licensed under the MPL v2.0.
