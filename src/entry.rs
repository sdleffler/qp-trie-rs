use core::borrow::Borrow;
use core::marker::PhantomData;
use core::mem;

use unreachable::UncheckedOptionExt;

use node::{Leaf, Node};
use util::nybble_get_mismatch;

pub fn make_entry<'a, K: 'a + Borrow<[u8]>, V: 'a>(
    key: K,
    root: &'a mut Option<Node<K, V>>,
    count: &'a mut usize,
) -> Entry<'a, K, V> {
    match *root {
        Some(..) => Entry::nonempty(key, root, count),
        None => Entry::empty(key, root, count),
    }
}

/// An entry - occupied or vacant - in the trie, corresponding to some given key.
#[derive(Debug)]
pub enum Entry<'a, K: 'a, V: 'a> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}

impl<'a, K: 'a + Borrow<[u8]>, V: 'a> Entry<'a, K, V> {
    fn nonempty(key: K, root: &'a mut Option<Node<K, V>>, count: &'a mut usize) -> Entry<'a, K, V> {
        let (exemplar_ptr, mismatch) = {
            let node = unsafe { root.as_mut().unchecked_unwrap() };
            let exemplar = node.get_exemplar_mut(key.borrow());
            let mismatch = nybble_get_mismatch(exemplar.key_slice(), key.borrow());
            (exemplar as *mut Leaf<K, V>, mismatch)
        };

        match mismatch {
            None => Entry::occupied(exemplar_ptr, root as *mut Option<Node<K, V>>, count),

            Some((b, i)) => {
                let node = unsafe { root.as_mut().unchecked_unwrap() };

                Entry::vacant_nonempty(key, i, b, node, count)
            }
        }
    }

    fn occupied(
        leaf: *mut Leaf<K, V>,
        root: *mut Option<Node<K, V>>,
        count: &'a mut usize,
    ) -> Entry<'a, K, V> {
        Entry::Occupied(OccupiedEntry {
            _dummy: PhantomData,
            leaf,
            root,
            count,
        })
    }

    fn vacant_nonempty(
        key: K,
        graft: usize,
        graft_nybble: u8,
        node: &'a mut Node<K, V>,
        count: &'a mut usize,
    ) -> Entry<'a, K, V> {
        Entry::Vacant(VacantEntry {
            key,
            inner: VacantEntryInner::Internal(graft, graft_nybble, node),
            count,
        })
    }

    fn empty(key: K, root: &'a mut Option<Node<K, V>>, count: &'a mut usize) -> Entry<'a, K, V> {
        Entry::Vacant(VacantEntry {
            key,
            inner: VacantEntryInner::Root(root),
            count,
        })
    }

    /// Get a mutable reference to a value already in the trie, if it exists - otherwise, insert a
    /// given default value, and return a mutable reference to its new location in the trie.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Vacant(vacant) => vacant.insert(default),
            Entry::Occupied(occupied) => occupied.into_mut(),
        }
    }

    /// Get a mutable reference to a value already in the trie, if it exists - otherwise, call the
    /// provided closure to construct a new value, insert it into the trie, and then return a
    /// mutable reference to it.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Vacant(vacant) => vacant.insert(default()),
            Entry::Occupied(occupied) => occupied.into_mut(),
        }
    }

    /// Get a reference to the key associated with this entry.
    pub fn key(&self) -> &K {
        match self {
            Entry::Vacant(vacant) => vacant.key(),
            Entry::Occupied(occupied) => occupied.key(),
        }
    }
}

/// A vacant entry in the trie.
#[derive(Debug)]
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    key: K,
    inner: VacantEntryInner<'a, K, V>,
    count: &'a mut usize,
}

#[derive(Debug)]
enum VacantEntryInner<'a, K: 'a, V: 'a> {
    Root(&'a mut Option<Node<K, V>>),
    Internal(usize, u8, &'a mut Node<K, V>),
}

impl<'a, K: 'a + Borrow<[u8]>, V: 'a> VacantEntry<'a, K, V> {
    /// Get a reference to the key associated with this vacant entry.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Consume the vacant entry to produce the associated key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Insert a value into the vacant entry, returning a mutable reference to the newly inserted
    /// value.
    pub fn insert(self, val: V) -> &'a mut V {
        *self.count += 1;
        match self.inner {
            VacantEntryInner::Root(root) => {
                debug_assert!(root.is_none());

                *root = Some(Node::Leaf(Leaf::new(self.key, val)));
                let root_mut_opt = root.as_mut();
                let leaf_mut = unsafe { root_mut_opt.unchecked_unwrap().unwrap_leaf_mut() };
                &mut leaf_mut.val
            }
            VacantEntryInner::Internal(graft, graft_nybble, node) => {
                node.insert_with_graft_point(graft, graft_nybble, self.key, val)
            }
        }
    }
}

/// An occupied entry in the trie.
#[derive(Debug)]
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    _dummy: PhantomData<&'a mut ()>,

    leaf: *mut Leaf<K, V>,
    root: *mut Option<Node<K, V>>,
    count: &'a mut usize,
}

impl<'a, K: 'a + Borrow<[u8]>, V: 'a> OccupiedEntry<'a, K, V> {
    /// Get a reference to the key of the entry.
    pub fn key(&self) -> &K {
        let leaf = unsafe { &*self.leaf };
        &leaf.key
    }

    /// Remove the entry from the trie, returning the stored key and value.
    pub fn remove_entry(self) -> (K, V) {
        let root = unsafe { &mut *self.root };
        *self.count -= 1;
        match *root {
            Some(Node::Leaf(_)) => {
                let leaf_opt = root.take();
                let leaf = unsafe { leaf_opt.unchecked_unwrap().unwrap_leaf() };

                debug_assert!(leaf.key_slice() == self.key().borrow());
                (leaf.key, leaf.val)
            }

            Some(Node::Branch(_)) => {
                let branch_opt = root.as_mut();
                let branch = unsafe { branch_opt.unchecked_unwrap() };

                let leaf_opt = branch.remove_validated(self.key().borrow());

                debug_assert!(leaf_opt.is_some());
                let leaf = unsafe { leaf_opt.unchecked_unwrap() };

                (leaf.key, leaf.val)
            }

            None => unsafe { debug_unreachable!() },
        }
    }

    /// Get a reference to the value in the occupied entry.
    pub fn get(&self) -> &V {
        let leaf = unsafe { &*self.leaf };
        &leaf.val
    }

    /// Get a mutable reference to the value in the occupied entry.
    pub fn get_mut(&mut self) -> &mut V {
        let leaf = unsafe { &mut *self.leaf };
        &mut leaf.val
    }

    /// Consume the entry to produce a mutable reference to the associated value.
    pub fn into_mut(self) -> &'a mut V {
        let leaf = unsafe { &mut *self.leaf };
        &mut leaf.val
    }

    /// Replace the associated value, returning the old one.
    pub fn insert(&mut self, val: V) -> V {
        let leaf = unsafe { &mut *self.leaf };
        mem::replace(&mut leaf.val, val)
    }

    /// Remove the entry altogether, returning the previously stored value.
    pub fn remove(self) -> V {
        self.remove_entry().1
    }
}
