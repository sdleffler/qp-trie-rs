extern crate rand;
#[macro_use]
extern crate quickcheck;

#[cfg(feature = "serde")]
extern crate bincode;
#[cfg(feature = "serde")]
extern crate serde_json;

extern crate qp_trie;

use quickcheck::TestResult;
use rand::seq::SliceRandom;
use std::collections::HashMap;

use qp_trie::*;

quickcheck! {
    fn insert_and_get(elts: Vec<(u8, u64)>) -> bool {
        let mut elts = elts;
        let mut rng = rand::thread_rng();
        elts.sort_by_key(|e| e.0);
        elts.dedup_by_key(|e| e.0);
        elts.shuffle(&mut rng);

        let hashmap: HashMap<u8, u64> = elts.iter().cloned().collect();
        let trie = {
            let mut trie = Trie::<[u8; 1], u64>::new();

            for (i, (b, s)) in elts.into_iter().enumerate() {
                assert_eq!(trie.count(), i);
                trie.insert([b], s);
            }

            trie
        };


        for (&key, &value) in hashmap.iter() {
            if trie.get(&[key]) != Some(&value) {
                return false;
            }
        }

        for (&key, &value) in trie.iter() {
            if hashmap[&key[0]] != value {
                return false;
            }
        }

        true
    }

    fn insert_and_remove(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
        let mut hashmap = HashMap::new();
        let mut trie = Trie::new();

        for &(ref k, v_opt) in &elts {
            match v_opt {
                Some(v) => {
                    hashmap.insert(k.as_ref(), v);
                    trie.insert(k.as_ref(), v);
                }
                None => {
                    hashmap.remove(&k.as_ref());
                    trie.remove(&k.as_ref());
                },
            }
        }

        let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

        hashmap == collected
    }

    fn prefix_sets(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
        let mut trie = Trie::new();

        for &(ref k, v) in elts.iter() {
            trie.insert(&k[..], v);
        }

        let filtered: HashMap<&[u8], u64> = trie.iter().filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
        let prefixed: HashMap<&[u8], u64> = trie.remove_prefix(&prefix[..]).into_iter().collect();

        filtered == prefixed
    }

    fn prefix_sets_ref(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
        let mut trie = Trie::new();

        for &(ref k, v) in elts.iter() {
            trie.insert(&k[..], v);
        }

        let filtered: HashMap<&[u8], u64> = trie.iter().filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
        let prefixed: HashMap<&[u8], u64> = trie.iter_prefix(&prefix[..]).map(|(&key, &val)| (key, val)).collect();
        let subtried: HashMap<&[u8], u64> = trie.subtrie(&prefix[..]).iter().map(|(&key, &val)| (key, val)).collect();

        filtered == prefixed && filtered == subtried
    }

    fn prefix_sets_mut(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
        let mut trie = Trie::new();

        for &(ref k, v) in elts.iter() {
            trie.insert(&k[..], v);
        }

        let filtered: HashMap<&[u8], u64> = trie.iter_mut().filter_map(|(&key, &mut val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
        let prefixed: HashMap<&[u8], u64> = trie.iter_prefix_mut(&prefix[..]).map(|(&key, &mut val)| (key, val)).collect();

        filtered == prefixed
    }

    fn entry_insert_and_remove(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
        let mut hashmap = HashMap::new();
        let mut trie = Trie::new();

        for &(ref k, v_opt) in &elts {
            match v_opt {
                Some(v) => {
                    hashmap.insert(k.as_ref(), v);

                    match trie.entry(k.as_ref()) {
                        Entry::Occupied(mut occupied) => { occupied.insert(v); }
                        Entry::Vacant(vacant) => { vacant.insert(v); }
                    }
                }
                None => {
                    hashmap.remove(&k.as_ref());

                    match trie.entry(k.as_ref()) {
                        Entry::Occupied(occupied) => { occupied.remove(); },
                        Entry::Vacant(..) => {},
                    }
                },
            }
        }

        let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

        hashmap == collected
    }

    fn longest_common_prefix(boolfix: Vec<bool>, boolts: Vec<(Vec<bool>, u64)>) -> TestResult {
        let prefix = boolfix.into_iter().map(|b| if b { 1 } else { 0 }).collect::<Vec<u8>>();
        let elts = boolts.into_iter().map(|(key, val)| (key.into_iter().map(|b| if b { 1 } else { 0 }).collect::<Vec<u8>>(), val)).collect::<Vec<(Vec<u8>, u64)>>();

        let mut trie = Trie::new();

        for &(ref k, v) in elts.iter() {
            trie.insert(&k[..], v);
        }

        let lcp = elts.iter().fold(&[][..], |lcp, (k, _)| {
            let mut i = 0;

            for (j, (b, c)) in k.iter().cloned().zip(prefix.iter().cloned()).enumerate() {
                if b != c {
                    break;
                }

                i = j + 1;
            }

            if i >= lcp.len() {
                &prefix[..i]
            } else {
                lcp
            }
        });

        TestResult::from_bool(lcp == trie.longest_common_prefix(prefix.as_slice()))
    }

    fn iter_keys(kvs : HashMap<Vec<u8>, usize>) -> bool {
        let mut given_keys: Vec<_> = kvs.keys().cloned().collect();
        let trie: Trie<_, _> = kvs.into_iter().collect();
        let mut yielded_keys: Vec<_> = trie.keys().cloned().collect();

        given_keys.sort();
        yielded_keys.sort();

        given_keys == yielded_keys
    }

    fn iter_values(kvs : HashMap<Vec<u8>, usize>) -> bool {
        let mut given_values: Vec<_> = kvs.values().cloned().collect();
        let trie: Trie<_, _> = kvs.into_iter().collect();
        let mut yielded_values: Vec<_> = trie.values().cloned().collect();

        given_values.sort_unstable();
        yielded_values.sort_unstable();

        given_values == yielded_values
    }

    #[cfg(feature = "serde")]
    fn serialize(kvs: Vec<(Vec<u8>, usize)>) -> bool {
        let original: Trie<Vec<u8>, usize> = kvs.into_iter().collect();
        let serialized = bincode::serialize(&original).unwrap();
        let deserialized: Trie<_, _> = bincode::deserialize(&serialized).unwrap();

        deserialized == original
    }

    #[cfg(feature = "serde")]
    fn serialize_emptied(kvs: Vec<(Vec<u8>, usize)>) -> bool {
        let mut trie: Trie<Vec<u8>, usize> = kvs.iter().cloned().collect();

        for (k, _) in kvs {
            trie.remove(&k);
        }

        let serialized = bincode::serialize(&trie).unwrap();
        let deserialized: Trie<Vec<u8>, usize> = bincode::deserialize(&serialized).unwrap();

        deserialized == Trie::new()
    }
}

fn entry_insert_and_remove_regression(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
    let mut hashmap = HashMap::new();
    let mut trie = Trie::new();

    for &(ref k, v_opt) in &elts {
        match v_opt {
            Some(v) => {
                hashmap.insert(k.as_ref(), v);

                match trie.entry(k.as_ref()) {
                    Entry::Occupied(mut occupied) => {
                        occupied.insert(v);
                    }
                    Entry::Vacant(vacant) => {
                        vacant.insert(v);
                    }
                }
            }
            None => {
                hashmap.remove(&k.as_ref());

                match trie.entry(k.as_ref()) {
                    Entry::Occupied(occupied) => {
                        occupied.remove();
                    }
                    Entry::Vacant(..) => {}
                }
            }
        }
    }

    let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

    hashmap == collected
}

#[test]
fn entry_insert_and_remove_1() {
    entry_insert_and_remove_regression(vec![
        (vec![83], Some(0)),
        (vec![83, 0], Some(0)),
        (vec![35], Some(0)),
    ]);
}

#[test]
fn entry_insert_and_remove_2() {
    entry_insert_and_remove_regression(vec![
        (vec![30], Some(0)),
        (vec![30, 0], Some(0)),
        (vec![13], Some(0)),
    ]);
}

fn prefix_sets_regression(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) {
    let mut trie = Trie::new();

    for &(ref k, v) in elts.iter() {
        trie.insert(&k[..], v);
    }

    let filtered: HashMap<&[u8], u64> = trie
        .iter()
        .filter_map(|(&key, &val)| {
            if key.starts_with(&prefix[..]) {
                Some((key, val))
            } else {
                None
            }
        })
        .collect();
    let prefixed: HashMap<&[u8], u64> = trie.remove_prefix(&prefix[..]).into_iter().collect();

    assert_eq!(filtered, prefixed);
}

#[test]
fn prefix_sets_1() {
    prefix_sets_regression(vec![], vec![(vec![], 0), (vec![0], 0)]);
}

fn insert_and_remove_regression(elts: Vec<(Vec<u8>, Option<u64>)>) {
    let mut hashmap = HashMap::new();
    let mut trie = Trie::new();

    for &(ref k, v_opt) in &elts {
        match v_opt {
            Some(v) => {
                hashmap.insert(k.as_ref(), v);
                trie.insert(k.as_ref(), v);
            }
            None => {
                hashmap.remove(&k.as_ref());
                trie.remove(&k.as_ref());
            }
        }
    }

    let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

    assert_eq!(hashmap, collected);
}

#[test]
fn insert_and_remove_1() {
    insert_and_remove_regression(vec![
        (vec![], Some(0)),
        (vec![46], Some(0)),
        (vec![62], None),
    ]);
}

fn insert_and_get_vec(elts: Vec<(u8, u64)>) {
    let hashmap: HashMap<u8, u64> = elts.iter().cloned().collect();
    let trie = {
        let mut trie = Trie::<[u8; 1], u64>::new();

        for (i, (b, s)) in elts.into_iter().enumerate() {
            assert_eq!(trie.count(), i);
            trie.insert([b], s);
        }

        trie
    };

    for (key, value) in hashmap {
        assert_eq!(trie.get(&[key]), Some(&value), "Sad trie: {:?}", trie,);
    }
}

#[test]
fn insert_and_get_1() {
    insert_and_get_vec(vec![(17, 0), (0, 0), (16, 0), (18, 0)]);
}

#[test]
fn insert_and_get_2() {
    insert_and_get_vec(vec![(5, 0), (0, 5), (1, 13), (49, 31)]);
}

#[test]
fn insert_and_get_3() {
    insert_and_get_vec(vec![(57, 0), (41, 0), (0, 0), (89, 0)]);
}

#[test]
fn insert_and_get_4() {
    insert_and_get_vec(vec![(3, 0), (35, 0), (0, 2), (13, 0)]);
}

#[test]
fn insert_and_get_5() {
    insert_and_get_vec(vec![(0, 0), (32, 9), (87, 5), (89, 26)]);
}

#[test]
fn longest_common_prefix_simple() {
    use wrapper::{BStr, BString};

    let mut trie = Trie::<BString, u32>::new();

    trie.insert("z".into(), 2);
    trie.insert("aba".into(), 5);
    trie.insert("abb".into(), 6);
    trie.insert("abc".into(), 50);

    let ab_sum = trie
        .iter_prefix(trie.longest_common_prefix(AsRef::<BStr>::as_ref("abd")))
        .fold(0, |acc, (_, &v)| {
            println!("Iterating over child: {:?}", v);

            acc + v
        });

    println!("{}", ab_sum);
    assert_eq!(ab_sum, 5 + 6 + 50);
}

#[test]
fn longest_common_prefix_complex() {
    use wrapper::{BStr, BString};

    let mut trie = Trie::<BString, u32>::new();

    trie.insert("z".into(), 2);
    trie.insert("aba".into(), 5);
    trie.insert("abb".into(), 6);
    trie.insert("abc".into(), 50);

    let ab_sum = trie
        .iter_prefix(trie.longest_common_prefix(AsRef::<BStr>::as_ref("abz")))
        .fold(0, |acc, (_, &v)| {
            println!("Iterating over child: {:?}", v);

            acc + v
        });

    println!("{}", ab_sum);
    assert_eq!(ab_sum, 5 + 6 + 50);
}

#[test]
#[cfg(feature = "serde")]
fn serialize_max_branching_factor() {
    let kvs = (0u16..256).map(|b| {
        let v = b as u8;
        let k: Vec<_> = (0..32).map(|i| v.wrapping_add(i)).collect();
        (k, v)
    });

    let original: Trie<Vec<u8>, u8> = kvs.collect();
    let serialized = bincode::serialize(&original).unwrap();
    let deserialized: Trie<_, _> = bincode::deserialize(&serialized).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
#[cfg(feature = "serde")]
fn serialize_pathological_branching() {
    use wrapper::BString;

    let kvs = (0..64).map(|length| {
        let seq = vec![0; length];
        let k = String::from_utf8(seq).unwrap();
        (BString::from(k), 0)
    });

    let original: Trie<BString, u8> = kvs.collect();
    let serialized = serde_json::to_vec(&original).unwrap();
    let deserialized: Trie<_, _> = serde_json::from_slice(&serialized).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn issue_22_regression_remove_prefix() {
    let mut trie = Trie::new();
    for i in 0..10 {
        let mut bytes = [0; 16];
        let (left, right) = bytes.split_at_mut(8);
        left.copy_from_slice(&u64::to_be_bytes(i));
        right.copy_from_slice(&u64::to_be_bytes(i));
        trie.insert(bytes, ());
    }
    assert_eq!(trie.count(), 10);
    for i in 0..5 {
        let subtrie = trie.remove_prefix(&u64::to_be_bytes(i)[..]);
        assert_eq!(subtrie.count(), 1);
    }
    assert_eq!(trie.count(), 5);
}

#[test]
fn issue_31_entry_count_decrement() {
    let mut trie = Trie::new();

    trie.insert_str("one", 1);
    assert_eq!(1, trie.count());

    match trie.entry("two".into()) {
        Entry::Occupied(ent) => panic!("'two' shouldn't exist yet {:?}", ent),
        Entry::Vacant(ent) => {
            ent.insert(2); // doesn't update `count`
        }
    }

    assert_eq!(2, trie.count());
}

#[test]
fn issue_36_node_count_after_clear() {
    let mut trie = Trie::new();
    trie.insert_str("one", 1);
    assert_eq!(1, trie.count());
    trie.clear();
    assert_eq!(0, trie.count());
}
