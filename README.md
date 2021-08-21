[![Build Status](https://travis-ci.org/sdleffler/qp-trie-rs.svg?branch=master)](https://travis-ci.org/sdleffler/qp-trie-rs)
[![Docs Status](https://docs.rs/qp-trie/badge.svg)](https://docs.rs/qp-trie)
[![On crates.io](https://img.shields.io/crates/v/qp-trie.svg)](https://crates.io/crates/qp-trie)

# qp-trie-rs: A QP-trie implementation in pure Rust

A QP-trie ("Quelques-bits Popcount trie" or "Quad-bit Popcount trie") is a
radix trie for keys which can be interpreted as a string of nybbles (where a
nybble is half a byte, or four bits.) QP-tries are essentially Patricia tries
which branch on nybbles instead of individual bits; as such, a QP-trie has a
branching factor (and radix) of 16.

## Serialization/deserialization through Serde

Optionally, the `qp_trie::Trie` type supports (de-)serialization through
[serde](https://github.com/serde-rs/serde). Enabling the `serde` feature will
enable compilation of `Deserialize` and `Serialize` implementations for `Trie`.

## When should I use a QP-trie?

QP-tries as implemented in this crate are key-value maps for any keys which
implement `qp_trie::AsKey`, a specialized trait akin to `Borrow<[u8]>`. They
are useful whenever you might need the same operations as a `HashMap` or
`BTreeMap`, but need either a bit more speed (QP-tries are as fast or a bit
faster as Rust's `HashMap` with the default hasher) and/or the ability to
efficiently query for sets of elements with a given prefix.

QP-tries support efficient lookup/insertion/removal of individual elements,
lookup/removal of sets of values with keys with a given prefix.

## Examples

Keys can be any type which implements `AsKey`. Currently, this means strings as
well as byte slices, vectors, and arrays. Here's a naive, simple example of
putting 9 2-element byte arrays into the trie, and then removing all byte
arrays which begin with "1":

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

Benchmarks are run against the `qptrie` crate, the Rust stdlib `BTreeMap`, and
the stdlib `HashMap` with both default and FNV hashing. `qp_trie::Trie`
consistently outperforms the `std::collections` `BTreeMap` and `HashMap`, as
well as the `qptrie` crate's implementation.

Benchmarks named `exotrie` are using the `qptrie::Trie` implementation.

```
test bench_btreemap_get      ... bench: 111,468,098 ns/iter (+/- 10,103,247)
test bench_btreemap_insert   ... bench: 112,124,846 ns/iter (+/- 14,296,195)
test bench_exotrie_get       ... bench:  46,195,582 ns/iter (+/- 16,943,561)
test bench_exotrie_insert    ... bench:  52,886,847 ns/iter (+/- 15,574,538)
test bench_fnvhashmap_get    ... bench:   9,530,109 ns/iter (+/- 820,763)
test bench_fnvhashmap_insert ... bench:  21,281,107 ns/iter (+/- 7,254,084)
test bench_hashmap_get       ... bench:  49,653,426 ns/iter (+/- 7,004,051)
test bench_hashmap_insert    ... bench:  47,771,824 ns/iter (+/- 4,979,606)
test bench_trie_get          ... bench:  40,898,914 ns/iter (+/- 13,400,062)
test bench_trie_insert       ... bench:  50,966,392 ns/iter (+/- 18,077,240)
```

## License

The `qp-trie-rs` crate is licensed under the MPL v2.0.
