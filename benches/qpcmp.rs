#![feature(test)]

extern crate qptrie;
extern crate qp_trie;
extern crate test;

use std::collections::{BTreeMap, HashMap};

use qptrie::Trie as ExoTrie;
use qp_trie::Trie;

use test::Bencher;


#[bench]
fn bench_trie_insert(b: &mut Bencher) {
    let mut trie = Trie::new();

    let a = 1_234u32;
    let mut x = 0u32;

    b.iter(move || for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    });
}


#[bench]
fn bench_trie_get(b: &mut Bencher) {
    let mut trie = Trie::new();

    let a = 1_234u32;
    let mut x = 0u32;

    for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    }

    b.iter(move || for _ in 0..499_979 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.get(&key[..]).unwrap();
    });
}


#[bench]
fn bench_exotrie_insert(b: &mut Bencher) {
    let mut trie = ExoTrie::default();

    let a = 1_234u32;
    let mut x = 0u32;

    b.iter(move || for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    });
}


#[bench]
fn bench_exotrie_get(b: &mut Bencher) {
    let mut trie = ExoTrie::default();

    let a = 1_234u32;
    let mut x = 0u32;

    for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    }

    b.iter(move || for _ in 0..499_979 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.get(&key).unwrap();
    });
}


#[bench]
fn bench_btreemap_insert(b: &mut Bencher) {
    let mut trie = BTreeMap::new();

    let a = 1_234u32;
    let mut x = 0u32;

    b.iter(move || for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    });
}


#[bench]
fn bench_btreemap_get(b: &mut Bencher) {
    let mut trie = BTreeMap::new();

    let a = 1_234u32;
    let mut x = 0u32;

    for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    }

    b.iter(move || for _ in 0..499_979 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.get(&key).unwrap();
    });
}


#[bench]
fn bench_hashmap_insert(b: &mut Bencher) {
    let mut trie = HashMap::new();

    let a = 1_234u32;
    let mut x = 0u32;

    b.iter(move || for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    });
}


#[bench]
fn bench_hashmap_get(b: &mut Bencher) {
    let mut trie = HashMap::new();

    let a = 1_234u32;
    let mut x = 0u32;

    for _ in 0..499_980 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.insert(key, ());
    }

    b.iter(move || for _ in 0..499_979 {
        x = (x + a) % 499_979;
        let key = [x as u8, (x >> 8) as u8, (x >> 16) as u8, (x >> 24) as u8];
        trie.get(&key).unwrap();
    });
}
