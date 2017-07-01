#[macro_use]
extern crate debug_unreachable;
extern crate unreachable;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
extern crate rand;

use std::borrow::Borrow;
use std::cmp;
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use unreachable::UncheckedOptionExt;


// Get the "nybble index" corresponding to the `n`th nybble in the given slice.
//
// This is `1 + b` where `b` is the `n`th nybble, unless the given slice has less than `n / 2`
// elements, in which case `0` is returned.
#[inline]
fn nybble_index(n: usize, slice: &[u8]) -> u8 {
    let byte_idx = n / 2;

    if byte_idx < slice.len() {
        let byte = slice[byte_idx];

        // If the index is even, return the lower nybble. Even, the higher nybble.
        // In both cases, increment by one. The zero-index is reserved for the "head" of the sparse
        // array.
        if n & 1 == 0 {
            1 + (byte & 0x0F)
        } else {
            1 + (byte >> 4)
        }
    } else {
        // If the nybble is out-of-range, we return zero. This is not some sort of weird
        // convention which would be clearer served by an `Option`; instead, we're actually
        // returning the "head" index of the sparse array. In the case that our trie `Branch` node
        // here - say it's branching at the `nth` nybble - contains a single entry of exactly `n /
        // 2` bytes long, then we have to have someplace to put it - the head. Essentially the head
        // is where leaf nodes which do not live at the fringes of the tree are stored.
        0
    }
}


// Find the nybble at which the two provided slices mismatch. If no such nybble exists and the
// slices are the same length, `None` is returned; if no such nybble exists but the slices are
// *not* the same length, then the point at which one slice has a byte and the other has ended is
// considered the mismatch point.
#[inline]
fn nybble_mismatch(left: &[u8], right: &[u8]) -> Option<usize> {
    let mut difference;

    for (i, (l, r)) in left.iter().cloned().zip(right.iter().cloned()).enumerate() {
        difference = l ^ r;

        if difference != 0 {
            if difference & 0x0F == 0 {
                return Some(1 + i * 2);
            } else {
                return Some(i * 2);
            }
        }
    }

    if left.len() == right.len() {
        None
    } else {
        Some(cmp::min(left.len(), right.len()) * 2)
    }
}


#[inline]
fn nybble_get_mismatch(left: &[u8], right: &[u8]) -> Option<(u8, usize)> {
    let mut difference;

    for (i, (l, r)) in left.iter().cloned().zip(right.iter().cloned()).enumerate() {
        difference = l ^ r;

        if difference != 0 {
            if difference & 0x0F == 0 {
                return Some((1 + (l >> 4), 1 + i * 2));
            } else {
                return Some((1 + (l & 0x0F), i * 2));
            }
        }
    }

    if left.len() == right.len() {
        None
    } else {
        let idx = cmp::min(left.len(), right.len()) * 2;

        Some((nybble_index(idx, left), idx))
    }
}


// A sparse array, holding up to 17 elements, indexed by nybbles with a special exception for
// elements which are shorter than the "choice point" of the branch node which holds this sparse
// array. This special exception is the "head".
struct Sparse<T> {
    index: u32,
    entries: Vec<T>,
}


impl<T: fmt::Debug> fmt::Debug for Sparse<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Sparse {{ index: {:b}, entries: {:?} }}",
            self.index,
            self.entries
        )
    }
}


impl<T> Sparse<T> {
    #[inline]
    fn new() -> Sparse<T> {
        Sparse {
            index: 0,
            entries: Vec::new(),
        }
    }


    #[inline]
    fn len(&self) -> usize {
        self.entries.len()
    }


    // Go from a nybble-index to an index in the internal element vector.
    #[inline]
    fn actual(&self, idx: u8) -> usize {
        (self.index & ((1 << idx) - 1)).count_ones() as usize
    }


    // Test whether or not the sparse array contains an element for the given index.
    #[inline]
    fn contains(&self, idx: u8) -> bool {
        self.index & (1 << idx) != 0
    }


    // Immutably borrow the corresponding element, if it exists.
    #[inline]
    fn get(&self, idx: u8) -> Option<&T> {
        if self.contains(idx) {
            Some(&self.entries[self.actual(idx)])
        } else {
            None
        }
    }


    // Mutably borrow the corresponding element, if it exists.
    #[inline]
    fn get_mut(&mut self, idx: u8) -> Option<&mut T> {
        if self.contains(idx) {
            let i = self.actual(idx);
            Some(&mut self.entries[i])
        } else {
            None
        }
    }


    // Immutably borrow the element corresponding to this index if it exists - otherwise, immutably
    // borrow an arbitrary element of the array.
    // TODO: Faster to not branch and just calculate the index and return it?
    #[inline]
    fn get_or_any(&self, idx: u8) -> &T {
        if self.contains(idx) {
            &self.entries[self.actual(idx)]
        } else {
            &self.entries[0]
        }
    }


    // Mutably borrow the element corresponding to this index if it exists - otherwise, mutably
    // borrow an arbitrary element of the array.
    // TODO: Faster to not branch and just calculate the index and return it?
    #[inline]
    fn get_or_any_mut(&mut self, idx: u8) -> &mut T {
        if self.contains(idx) {
            let i = self.actual(idx);
            &mut self.entries[i]
        } else {
            &mut self.entries[0]
        }
    }


    // Assuming that the array does not already contain an element for this index, insert the
    // given element.
    #[inline]
    fn insert(&mut self, idx: u8, elt: T) -> &mut T {
        debug_assert!(!self.contains(idx));
        let i = self.actual(idx);
        self.index |= 1 << idx;
        self.entries.insert(i, elt);
        &mut self.entries[i]
    }


    // Assuming that the array contains this index, remove that index and return the corresponding
    // element.
    #[inline]
    fn remove(&mut self, idx: u8) -> T {
        debug_assert!(self.contains(idx));
        let i = self.actual(idx);
        self.index &= !(1 << idx);
        self.entries.remove(i)
    }


    // Clear the array, assuming it has a single element remaining, and return that element.
    #[inline]
    fn clear_last(&mut self) -> T {
        debug_assert!(self.len() == 1);
        unsafe { self.entries.pop().unchecked_unwrap() }
    }
}


// A leaf in the trie.
struct Leaf<K: ToOwned, V> {
    key: K::Owned,
    val: V,
}


impl<K: ToOwned, V> Leaf<K, V> {
    #[inline]
    fn new(key: K, val: V) -> Leaf<K, V> {
        Leaf {
            key: key.to_owned(),
            val,
        }
    }
}


impl<K: ToOwned + Borrow<[u8]>, V> Leaf<K, V> {
    #[inline]
    fn key_slice(&self) -> &[u8] {
        self.key.borrow().borrow()
    }
}


// A branch node in the QP-trie. It contains up to 17 entries, only 16 of which may actually be
// other branches - the 0th entry, if it exists in the sparse array, is the "head" of the branch,
// containing a key/value pair corresponding to the leaf which would otherwise occupy the location
// of the branch in the trie.
struct Branch<K: ToOwned, V> {
    // The nybble that this `Branch` cares about. Entries in the `entries` sparse array correspond
    // to different values of the nybble at the choice point for given keys.
    choice: usize,
    entries: Sparse<Node<K, V>>,
}


impl<K: ToOwned, V: fmt::Debug> fmt::Debug for Branch<K, V>
where
    K::Owned: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Branch")
            .field("choice", &self.choice)
            .field("entries", &self.entries)
            .finish()
    }
}


impl<K: ToOwned + Borrow<[u8]>, V> Branch<K, V> {
    // Create an empty `Branch` with the given choice point.
    #[inline]
    fn new(choice: usize) -> Branch<K, V> {
        Branch {
            choice,
            entries: Sparse::new(),
        }
    }


    // Return the nybble index corresponding to the branch's choice point in the given key.
    #[inline]
    fn index(&self, key: &[u8]) -> u8 {
        nybble_index(self.choice, key)
    }


    // Returns true if and only if the `Branch` has only one child. This is used for determining
    // whether or not to replace a branch with its only child.
    #[inline]
    fn is_singleton(&self) -> bool {
        self.entries.len() == 1
    }


    #[inline]
    fn has_entry(&self, index: u8) -> bool {
        self.entries.contains(index)
    }


    #[inline]
    fn entry_mut(&mut self, index: u8) -> &mut Node<K, V> {
        let entry = self.entries.get_mut(index);
        debug_assert!(entry.is_some());
        unsafe { entry.unchecked_unwrap() }
    }


    // Get the child node corresponding to the given key.
    #[inline]
    fn child(&self, key: &[u8]) -> Option<&Node<K, V>> {
        self.entries.get(nybble_index(self.choice, key.borrow()))
    }


    // Mutable version of `Branch::child`.
    #[inline]
    fn child_mut(&mut self, key: &[u8]) -> Option<&mut Node<K, V>> {
        self.entries.get_mut(
            nybble_index(self.choice, key.borrow()),
        )
    }


    // Immutably borrow the leaf for the given key, if it exists, mutually recursing through
    // `Node::get`.
    #[inline]
    fn get(&self, key: &[u8]) -> Option<&Leaf<K, V>> {
        match self.child(key.borrow()) {
            Some(child) => child.get(key),
            None => None,
        }
    }


    // Mutably borrow the value for the given key, if it exists, mutually recursing through
    // `Node::get_mut`.
    #[inline]
    fn get_mut(&mut self, key: &[u8]) -> Option<&mut Leaf<K, V>> {
        self.child_mut(key.borrow()).and_then(
            |node| node.get_mut(key),
        )
    }


    // Retrieve the node which contains the exemplar. This does not recurse and return the actual
    // exemplar - just the node which might be or contain it.
    #[inline]
    fn exemplar(&self, key: &[u8]) -> &Node<K, V> {
        self.entries.get_or_any(
            nybble_index(self.choice, key.borrow()),
        )
    }


    // As `Branch::exemplar` but for mutable borrows.
    #[inline]
    fn exemplar_mut(&mut self, key: &[u8]) -> &mut Node<K, V> {
        self.entries.get_or_any_mut(
            nybble_index(self.choice, key.borrow()),
        )
    }


    // Immutably borrow the exemplar for the given key, mutually recursing through
    // `Node::get_exemplar`.
    #[inline]
    fn get_exemplar(&self, key: &[u8]) -> &Leaf<K, V> {
        self.exemplar(key.borrow()).get_exemplar(key)
    }


    // Mutably borrow the exemplar for the given key, mutually recursing through
    // `Node::get_exemplar_mut`.
    #[inline]
    fn get_exemplar_mut(&mut self, key: &[u8]) -> &mut Leaf<K, V> {
        self.exemplar_mut(key.borrow()).get_exemplar_mut(key)
    }


    // Convenience method for inserting a leaf into the branch's sparse array.
    #[inline]
    fn insert_leaf(&mut self, leaf: Leaf<K, V>) -> &mut Leaf<K, V> {
        self.entries
            .insert(
                nybble_index(self.choice, leaf.key_slice()),
                Node::Leaf(leaf),
            )
            .unwrap_leaf_mut()
    }


    // Convenience method for inserting a branch into the branch's sparse array.
    #[inline]
    fn insert_branch(&mut self, index: u8, branch: Branch<K, V>) -> &mut Branch<K, V> {
        self.entries
            .insert(index, Node::Branch(branch))
            .unwrap_branch_mut()
    }


    // Assuming that the provided index is valid, remove the node with that nybble index and
    // return it.
    #[inline]
    fn remove(&mut self, index: u8) -> Node<K, V> {
        self.entries.remove(index)
    }


    // Assuming that the branch node has only one element back, remove it and return it in
    // preparation for replacement with a leaf.
    #[inline]
    fn clear_last(&mut self) -> Node<K, V> {
        self.entries.clear_last()
    }
}


impl<K: ToOwned, V> Branch<K, V> {
    // Count the number of entries stored in this branch. This traverses all subnodes of the
    // branch, so it is relatively expensive.
    #[inline]
    fn count(&self) -> usize {
        self.entries.entries.iter().map(Node::count).sum()
    }
}


// A node in the trie. `K` must be `ToOwned` because the `Owned` version is what we store.
enum Node<K: ToOwned, V> {
    Leaf(Leaf<K, V>),
    Branch(Branch<K, V>),
}


impl<K: ToOwned, V: fmt::Debug> fmt::Debug for Node<K, V>
where
    K::Owned: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Leaf(ref leaf) => {
                f.debug_struct("Leaf")
                    .field("key", &leaf.key)
                    .field("val", &leaf.val)
                    .finish()
            }
            Node::Branch(ref branch) => {
                f.debug_struct("Branch")
                    .field("choice", &branch.choice)
                    .field("entries", &branch.entries)
                    .finish()
            }
        }
    }
}


impl<K: ToOwned + Borrow<[u8]>, V> Node<K, V> {
    // The following `unwrap_` functions are used for (at times) efficiently circumventing the
    // borrowchecker. All of them use `debug_unreachable!` internally, which means that in release,
    // a misuse can cause undefined behavior (because the tried-to-unwrap-wrong-thing code path is
    // likely to be statically eliminated.)

    #[inline]
    fn unwrap_leaf(self) -> Leaf<K, V> {
        match self {
            Node::Leaf(leaf) => leaf,
            Node::Branch(..) => unsafe { debug_unreachable!() },
        }
    }

    #[inline]
    fn unwrap_leaf_ref(&self) -> &Leaf<K, V> {
        match *self {
            Node::Leaf(ref leaf) => leaf,
            Node::Branch(..) => unsafe { debug_unreachable!() },
        }
    }

    #[inline]
    fn unwrap_leaf_mut(&mut self) -> &mut Leaf<K, V> {
        match *self {
            Node::Leaf(ref mut leaf) => leaf,
            Node::Branch(..) => unsafe { debug_unreachable!() },
        }
    }

    #[inline]
    fn unwrap_branch_ref(&self) -> &Branch<K, V> {
        match *self {
            Node::Leaf(..) => unsafe { debug_unreachable!() },
            Node::Branch(ref branch) => branch,
        }
    }


    #[inline]
    fn unwrap_branch_mut(&mut self) -> &mut Branch<K, V> {
        match *self {
            Node::Leaf(..) => unsafe { debug_unreachable!() },
            Node::Branch(ref mut branch) => branch,
        }
    }


    // Borrow the associated leaf for a given key, if it exists in the trie.
    fn get(&self, key: &[u8]) -> Option<&Leaf<K, V>> {
        match *self {
            Node::Leaf(ref leaf) if leaf.key_slice() == key => Some(leaf),
            Node::Leaf(..) => None,

            Node::Branch(ref branch) => branch.get(key),
        }
    }


    // Mutably borrow the associated leaf for a given key, if it exists in the trie.
    fn get_mut(&mut self, key: &[u8]) -> Option<&mut Leaf<K, V>> {
        match *self {
            Node::Leaf(ref mut leaf) if leaf.key_slice() == key => Some(leaf),
            Node::Leaf(..) => None,

            Node::Branch(ref mut branch) => branch.get_mut(key),
        }
    }


    // Borrow the "exemplar" for a given key, if it exists. The exemplar is any leaf which exists
    // as a child of the same branch that the given key would be inserted into. This is necessary
    // to decide whether or not a new value for the given key can be inserted into an arbitrary
    // branch in the trie, as otherwise the invariant of branch choice points strictly increasing
    // with depth may be violated.
    //
    // If the key already exists in the trie, then the leaf containing it is returned as the
    // exemplar.
    fn get_exemplar(&self, key: &[u8]) -> &Leaf<K, V> {
        match *self {
            Node::Leaf(ref leaf) => leaf,
            Node::Branch(ref branch) => branch.get_exemplar(key),
        }
    }


    // Mutably borrow the exemplar for a given key.
    fn get_exemplar_mut(&mut self, key: &[u8]) -> &mut Leaf<K, V> {
        match *self {
            Node::Leaf(ref mut leaf) => leaf,
            Node::Branch(ref mut branch) => branch.get_exemplar_mut(key),
        }
    }


    // Borrow the node which contains all and only entries with keys beginning with
    // `prefix`, assuming there exists at least one such entry.
    //
    // PRECONDITION:
    // - There exists at least one node in the trie with the given prefix.
    fn get_prefix_validated<'a>(&'a self, prefix: &[u8]) -> &'a Node<K, V> {
        match *self {
            Node::Leaf(..) => self,
            Node::Branch(ref branch) => {
                if branch.choice >= prefix.len() * 2 {
                    self
                } else {
                    let child_opt = branch.child(prefix);
                    let child = unsafe { child_opt.unchecked_unwrap() };
                    child.get_prefix_validated(prefix)
                }
            }
        }
    }


    // Borrow the node which contains all and only entries with keys beginning with
    // `prefix`.
    fn get_prefix<'a>(&'a self, prefix: &[u8]) -> Option<&'a Node<K, V>> {
        match *self {
            Node::Leaf(ref leaf) if leaf.key_slice().starts_with(prefix) => Some(self),
            Node::Branch(ref branch)
                if branch.get_exemplar(prefix).key_slice().starts_with(prefix) => Some(
                self.get_prefix_validated(prefix),
            ),

            _ => None,
        }
    }


    // Mutably borrow the node which contains all and only entries with keys beginning with
    // `prefix`, assuming there exists at least one such entry.
    //
    // PRECONDITION:
    // - There exists at least one node in the trie with the given prefix.
    fn get_prefix_validated_mut<'a>(&'a mut self, prefix: &[u8]) -> &'a mut Node<K, V> {
        match *self {
            Node::Leaf(..) => self,
            Node::Branch(..) => {
                if self.unwrap_branch_mut().choice >= prefix.len() * 2 {
                    self
                } else {
                    let child_opt = self.unwrap_branch_mut().child_mut(prefix);
                    let child = unsafe { child_opt.unchecked_unwrap() };

                    child.get_prefix_validated_mut(prefix)
                }
            }
        }
    }


    // Mutably borrow the node which contains all and only entries with keys beginning with
    // `prefix`.
    fn get_prefix_mut<'a>(&'a mut self, prefix: &[u8]) -> Option<&'a mut Node<K, V>> {
        match *self {
            Node::Leaf(..) => {
                if self.unwrap_leaf_ref().key_slice().starts_with(prefix) {
                    Some(self)
                } else {
                    None
                }
            }

            Node::Branch(..) => {
                let has_prefix = {
                    let exemplar = self.unwrap_branch_ref().get_exemplar(prefix);

                    exemplar.key_slice().starts_with(prefix)
                };


                if has_prefix {
                    Some(self.get_prefix_validated_mut(prefix))
                } else {
                    None
                }
            }
        }
    }


    // Insert into the trie with a given "graft point" - the first point of nybble mismatch
    // between the key and an "exemplar" key.
    //
    // PRECONDITION:
    // - The key is not already in the trie.
    fn insert_with_graft_point(
        &mut self,
        graft: usize,
        graft_nybble: u8,
        key: K,
        val: V,
    ) -> &mut V {
        match *self {
            Node::Branch(ref mut branch) if branch.choice <= graft => {
                let index = branch.index(key.borrow());

                if branch.has_entry(index) {
                    branch.entry_mut(index).insert_with_graft_point(
                        graft,
                        graft_nybble,
                        key,
                        val,
                    )
                } else {
                    &mut branch.insert_leaf(Leaf::new(key, val)).val
                }
            }

            _ => {
                let node = mem::replace(self, Node::Branch(Branch::new(graft)));
                let graft_branch = self.unwrap_branch_mut();

                match node {
                    Node::Leaf(leaf) => {
                        graft_branch.insert_leaf(leaf);
                    }
                    Node::Branch(branch) => {
                        graft_branch.insert_branch(graft_nybble, branch);
                    }
                }

                &mut graft_branch.insert_leaf(Leaf::new(key, val)).val
            }
        }
    }


    // Insert a node into a nonempty trie.
    fn insert(&mut self, key: K, val: V) -> Option<V> {
        match *self {
            Node::Leaf(..) => {
                match nybble_mismatch(self.unwrap_leaf_ref().key_slice(), key.borrow()) {
                    None => Some(mem::replace(&mut self.unwrap_leaf_mut().val, val)),
                    Some(mismatch) => {
                        let leaf = mem::replace(self, Node::Branch(Branch::new(mismatch)))
                            .unwrap_leaf();
                        let branch = self.unwrap_branch_mut();

                        branch.insert_leaf(Leaf::new(key, val));
                        branch.insert_leaf(leaf);

                        None
                    }
                }
            }

            Node::Branch(..) => {
                let (mismatch, mismatch_nybble) = {
                    let exemplar = self.get_exemplar_mut(key.borrow());

                    let mismatch_opt = nybble_mismatch(exemplar.key_slice(), key.borrow());

                    match mismatch_opt {
                        Some(mismatch) => (mismatch, nybble_index(mismatch, exemplar.key_slice())),
                        None => return Some(mem::replace(&mut exemplar.val, val)),
                    }
                };

                self.insert_with_graft_point(mismatch, mismatch_nybble, key, val);

                None
            }
        }
    }


    // `remove_validated` assumes that it is being called on a `Node::Branch`.
    //
    // PRECONDITION:
    // - `self` is of the `Node::Branch` variant.
    fn remove_validated(&mut self, key: &[u8]) -> Option<Leaf<K, V>> {
        match *self {
            Node::Leaf(..) => unsafe { debug_unreachable!() },
            Node::Branch(..) => {
                let leaf = {
                    let branch = self.unwrap_branch_mut();
                    let index = branch.index(key);

                    match branch.child_mut(key) {
                        // Removing a leaf means waiting for `self` to be available so we can try
                        // to compress. Also we can't remove in this match arm since `branch` is
                        // borrowed.
                        Some(&mut Node::Leaf(ref leaf)) if leaf.key_slice() == key => {}

                        Some(child @ &mut Node::Branch(..)) => return child.remove_validated(key),
                        _ => return None,
                    };

                    branch.remove(index).unwrap_leaf()
                };

                // We removed a leaf. The branch's arity has reduced - we may be able to compress.
                if self.unwrap_branch_mut().is_singleton() {
                    let node = self.unwrap_branch_mut().clear_last();
                    mem::replace(self, node);
                }

                Some(leaf)
            }
        }
    }


    // Remove a node from the trie with the given key and return its value, if it exists.
    fn remove(root: &mut Option<Node<K, V>>, key: &[u8]) -> Option<Leaf<K, V>> {
        match *root {
            Some(Node::Leaf(..))
                if unsafe { root.as_ref().unchecked_unwrap() }
                       .unwrap_leaf_ref()
                       .key_slice() == key => {
                Some(unsafe { root.take().unchecked_unwrap() }.unwrap_leaf())
            }

            Some(ref mut node @ Node::Branch(..)) => node.remove_validated(key),

            _ => None,
        }
    }


    // `remove_prefix_validated` assumes that it is being called on a `Node::Branch`, and also
    // that there exists at least one node with the given prefix.
    //
    // PRECONDITION:
    // - There exists a node in the trie with the given prefix.
    // - `self` is of the `Branch` variant.
    fn remove_prefix_validated(&mut self, prefix: &[u8]) -> Option<Node<K, V>> {
        match *self {
            Node::Leaf(..) => unsafe { debug_unreachable!() },
            Node::Branch(..) => {
                let prefix_node = {
                    let branch = self.unwrap_branch_mut();
                    let index = branch.index(prefix);

                    match branch.child_mut(prefix) {
                        // Similar borrow logistics to `remove_validated`.
                        Some(&mut Node::Leaf(ref l)) if l.key_slice().starts_with(prefix) => {}
                        Some(&mut Node::Branch(ref child_branch))
                            if child_branch.choice >= prefix.len() * 2 => {}

                        Some(child @ &mut Node::Branch(..)) => {
                            return child.remove_prefix_validated(prefix)
                        }

                        _ => return None,
                    }

                    branch.remove(index)
                };

                if self.unwrap_branch_mut().is_singleton() {
                    let node = self.unwrap_branch_mut().clear_last();
                    mem::replace(self, node);
                }

                Some(prefix_node)
            }
        }
    }


    // Remove the node which holds all and only elements starting with the given prefix and return
    // it, if it exists.
    fn remove_prefix(root: &mut Option<Node<K, V>>, prefix: &[u8]) -> Option<Node<K, V>> {
        match *root {
            Some(Node::Leaf(..))
                if unsafe { root.as_ref().unchecked_unwrap() }
                       .unwrap_leaf_ref()
                       .key_slice()
                       .starts_with(prefix) => root.take(),

            Some(Node::Branch(..))
                if unsafe { root.as_ref().unchecked_unwrap() }
                       .unwrap_branch_ref()
                       .get_exemplar(prefix)
                       .key_slice()
                       .starts_with(prefix) => {
                if unsafe { root.as_ref().unchecked_unwrap() }
                    .unwrap_branch_ref()
                    .choice >= prefix.len()
                {
                    root.take()
                } else {
                    unsafe { root.as_mut().unchecked_unwrap() }.remove_prefix_validated(prefix)
                }
            }

            _ => None,
        }
    }
}


impl<K: ToOwned, V> Node<K, V> {
    fn count(&self) -> usize {
        match *self {
            Node::Leaf(..) => 1,
            Node::Branch(ref branch) => branch.count(),
        }
    }


    fn iter(&self) -> Iter<K, V> {
        Iter { stack: vec![self] }
    }
}


/// An iterator over the keys and values in a QP-trie.
pub struct IntoIter<K: ToOwned, V> {
    stack: Vec<Node<K, V>>,
}


impl<K: ToOwned, V> IntoIter<K, V> {
    fn root(node: Node<K, V>) -> IntoIter<K, V> {
        IntoIter { stack: vec![node] }
    }


    fn empty() -> IntoIter<K, V> {
        IntoIter { stack: vec![] }
    }
}


impl<K: ToOwned, V> Iterator for IntoIter<K, V> {
    type Item = (K::Owned, V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(Node::Leaf(leaf)) => Some((leaf.key, leaf.val)),
            Some(Node::Branch(branch)) => {
                self.stack.extend(branch.entries.entries.into_iter().rev());
                self.next()
            }
            None => None,
        }
    }
}


/// An iterator over immutable references to keys and values in a QP-trie.
pub struct Iter<'a, K: 'a + ToOwned, V: 'a> {
    stack: Vec<&'a Node<K, V>>,
}


impl<'a, K: ToOwned, V> Iter<'a, K, V> {
    fn root(node: &'a Node<K, V>) -> Iter<'a, K, V> {
        Iter { stack: vec![node] }
    }


    fn empty() -> Iter<'a, K, V> {
        Iter { stack: vec![] }
    }
}


impl<'a, K: 'a + ToOwned, V: 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(&Node::Leaf(ref leaf)) => Some((leaf.key.borrow(), &leaf.val)),
            Some(&Node::Branch(ref branch)) => {
                self.stack.extend(branch.entries.entries.iter().rev());
                self.next()
            }
            None => None,
        }
    }
}


/// An iterator over immutable references to keys and mutable references to values in a QP-trie.
pub struct IterMut<'a, K: 'a + ToOwned, V: 'a> {
    stack: Vec<&'a mut Node<K, V>>,
}


impl<'a, K: ToOwned, V> IterMut<'a, K, V> {
    fn root(node: &'a mut Node<K, V>) -> IterMut<'a, K, V> {
        IterMut { stack: vec![node] }
    }


    fn empty() -> IterMut<'a, K, V> {
        IterMut { stack: vec![] }
    }
}


impl<'a, K: 'a + ToOwned, V: 'a> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(&mut Node::Leaf(ref mut leaf)) => Some((leaf.key.borrow(), &mut leaf.val)),
            Some(&mut Node::Branch(ref mut branch)) => {
                self.stack.extend(branch.entries.entries.iter_mut().rev());
                self.next()
            }
            None => None,
        }
    }
}


/// An entry - occupied or vacant - in the trie, corresponding to some given key.
pub enum Entry<'a, K: 'a + ToOwned, V: 'a> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}


impl<'a, K: 'a + ToOwned + Borrow<[u8]>, V: 'a> Entry<'a, K, V> {
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
        match *self {
            Entry::Vacant(ref vacant) => vacant.key(),
            Entry::Occupied(ref occupied) => occupied.key(),
        }
    }
}


/// A vacant entry in the trie.
pub struct VacantEntry<'a, K: 'a + ToOwned, V: 'a> {
    key: K,
    inner: VacantEntryInner<'a, K, V>,
}


enum VacantEntryInner<'a, K: 'a + ToOwned, V: 'a> {
    Root(&'a mut Option<Node<K, V>>),
    Internal(usize, u8, &'a mut Node<K, V>),
}


impl<'a, K: 'a + ToOwned + Borrow<[u8]>, V: 'a> VacantEntry<'a, K, V> {
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
        match self.inner {
            VacantEntryInner::Root(root) => {
                debug_assert!(root.is_none());

                *root = Some(Node::Leaf(Leaf::new(self.key, val)));
                let root_mut_opt = root.as_mut();
                let node_mut = unsafe { root_mut_opt.unchecked_unwrap() };
                &mut node_mut.unwrap_leaf_mut().val
            }
            VacantEntryInner::Internal(graft, graft_nybble, node) => {
                node.insert_with_graft_point(graft, graft_nybble, self.key, val)
            }
        }
    }
}


/// An occupied entry in the trie.
pub struct OccupiedEntry<'a, K: 'a + ToOwned, V: 'a> {
    _dummy: PhantomData<&'a mut ()>,

    leaf: *mut Leaf<K, V>,
    root: *mut Option<Node<K, V>>,
}


impl<'a, K: 'a + ToOwned + Borrow<[u8]>, V: 'a> OccupiedEntry<'a, K, V> {
    /// Get a reference to the key of the entry.
    pub fn key(&self) -> &K {
        let leaf = unsafe { &*self.leaf };
        leaf.key.borrow()
    }


    /// Remove the entry from the trie, returning the stored key and value.
    pub fn remove_entry(self) -> (K::Owned, V) {
        let root = unsafe { &mut *self.root };

        match *root {
            Some(Node::Leaf(..)) => {
                let leaf_opt = root.take();
                let leaf = unsafe { leaf_opt.unchecked_unwrap() }.unwrap_leaf();

                debug_assert!(leaf.key_slice() == self.key().borrow());
                (leaf.key, leaf.val)
            }

            Some(Node::Branch(..)) => {
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


/// A QP-trie. QP stands for - depending on who you ask - either "quelques-bits popcount" or
/// "quad-bit popcount". In any case, the fact of the matter is that this is a compressed radix
/// trie with a branching factor of 16. It acts as a key-value map where the keys are any value
/// which can be converted to a slice of bytes.
///
/// It *can* be used with strings, although it is worth noting how inconvenient doing so is - as
/// some `&str` does not hash to the same value as that same `&str` converted to an `&[u8]`
/// (currently - see issue https://github.com/rust-lang/rust/issues/27108 which unfortunately
/// appears to be abandoned.) The following example uses strings and also showcases the necessary
/// inconveniences.
///
/// # Example
///
/// ```rust
/// # use qp_trie::Trie;
///
/// let mut trie = Trie::new();
///
/// trie.insert("abbc".as_bytes(), 1);
/// trie.insert("abcd".as_bytes(), 2);
/// trie.insert("bcde".as_bytes(), 3);
/// trie.insert("bdde".as_bytes(), 4);
/// trie.insert("bddf".as_bytes(), 5);
///
/// // This will print the following string:
/// //
/// // {[97, 98, 98, 99]: 1, [97, 98, 99, 100]: 2, [98, 99, 100, 101]: 3, [98, 100, 100, 101]: 4, [98, 100, 100, 102]: 5}
/// //
/// // Unfortunately, for Reasons, we cannot debug-print strings in this way (because strings do
/// // implement `Borrow<[u8]>` due to "hashing differences".) So in debug we get a list of byte
/// // values.
///
/// println!("{:?}", trie);
/// # assert_eq!(format!("{:?}", trie), "{[97, 98, 98, 99]: 1, [97, 98, 99, 100]: 2, [98, 99, 100, 101]: 3, [98, 100, 100, 101]: 4, [98, 100, 100, 102]: 5}");
///
/// assert_eq!(trie.get("abcd".as_bytes()), Some(&2));
/// assert_eq!(trie.get("bcde".as_bytes()), Some(&3));
///
/// // We can take subtries, removing all elements of the trie with a given prefix.
/// let mut subtrie = trie.remove_prefix("b".as_bytes());
///
/// assert_eq!(trie.get("abbc".as_bytes()), Some(&1));
/// assert_eq!(trie.get("abcd".as_bytes()), Some(&2));
/// assert_eq!(trie.get("bcde".as_bytes()), None);
/// assert_eq!(trie.get("bdde".as_bytes()), None);
/// assert_eq!(trie.get("bddf".as_bytes()), None);
///
/// assert_eq!(subtrie.get("abbc".as_bytes()), None);
/// assert_eq!(subtrie.get("abcd".as_bytes()), None);
/// assert_eq!(subtrie.get("bcde".as_bytes()), Some(&3));
/// assert_eq!(subtrie.get("bdde".as_bytes()), Some(&4));
/// assert_eq!(subtrie.get("bddf".as_bytes()), Some(&5));
///
/// // We can remove elements:
/// assert_eq!(trie.remove("abbc".as_bytes()), Some(1));
/// assert_eq!(trie.get("abbc".as_bytes()), None);
///
/// // We can mutate values:
/// *subtrie.get_mut("bdde".as_bytes()).unwrap() = 0;
/// assert_eq!(subtrie.get("bdde".as_bytes()), Some(&0));
/// ```
pub struct Trie<K: ToOwned, V> {
    root: Option<Node<K, V>>,
}


impl<K: fmt::Debug + ToOwned, V: fmt::Debug> fmt::Debug for Trie<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.root {
            Some(ref node) => f.debug_map().entries(node.iter()).finish(),
            None => f.debug_map().finish(),
        }
    }
}


impl<K: ToOwned, V> IntoIterator for Trie<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K::Owned, V);

    fn into_iter(self) -> Self::IntoIter {
        match self.root {
            Some(node) => IntoIter::root(node),
            None => IntoIter::empty(),
        }
    }
}


impl<K: ToOwned + Borrow<[u8]>, V> FromIterator<(K, V)> for Trie<K, V> {
    fn from_iter<I>(iterable: I) -> Trie<K, V>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut trie = Trie::new();

        for (key, val) in iterable {
            trie.insert(key, val);
        }

        trie
    }
}


impl<K: ToOwned + Borrow<[u8]>, V> Trie<K, V> {
    /// Create a new, empty trie.
    pub fn new() -> Trie<K, V> {
        Trie { root: None }
    }


    /// Iterate over all elements in the trie.
    pub fn iter(&self) -> Iter<K, V> {
        match self.root {
            Some(ref node) => Iter::root(node),
            None => Iter::empty(),
        }
    }


    /// Iterate over all elements in the trie, given a mutable reference to the associated value.
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        match self.root {
            Some(ref mut node) => IterMut::root(node),
            None => IterMut::empty(),
        }
    }


    /// Iterate over all elements with a given prefix.
    pub fn iter_prefix<L: Borrow<[u8]>>(&self, prefix: L) -> Iter<K, V> {
        match self.root.as_ref().and_then(
            |node| node.get_prefix(prefix.borrow()),
        ) {
            Some(node) => Iter::root(node),
            None => Iter::empty(),
        }
    }


    /// Iterate over all elements with a given prefix, but given a mutable reference to the
    /// associated value.
    pub fn iter_prefix_mut<L: Borrow<[u8]>>(&mut self, prefix: L) -> IterMut<K, V> {
        match self.root.as_mut().and_then(|node| {
            node.get_prefix_mut(prefix.borrow())
        }) {
            Some(node) => IterMut::root(node),
            None => IterMut::empty(),
        }
    }


    /// Count the number of entries in the tree. This is currently slow - it traverses the entire
    /// trie!
    ///
    /// TODO: Speed this up by tracking the size of the trie for each insert/removal.
    pub fn count(&self) -> usize {
        self.root.as_ref().map(Node::count).unwrap_or(0)
    }


    /// Get an immutable reference to the value associated with a given key, if it is in the tree.
    pub fn get<L: Borrow<[u8]>>(&self, key: L) -> Option<&V> {
        self.root
            .as_ref()
            .and_then(|node| node.get(key.borrow()))
            .map(|leaf| &leaf.val)
    }


    /// Get a mutable reference to the value associated with a given key, if it is in the tree.
    pub fn get_mut<L: Borrow<[u8]>>(&mut self, key: L) -> Option<&mut V> {
        self.root
            .as_mut()
            .and_then(|node| node.get_mut(key.borrow()))
            .map(|leaf| &mut leaf.val)
    }


    /// Insert a key/value pair into the trie, returning the old value if an entry already existed.
    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        match self.root {
            Some(ref mut root) => root.insert(key, val),
            None => {
                self.root = Some(Node::Leaf(Leaf::new(key, val)));
                None
            }
        }
    }


    /// Remove the key/value pair associated with a given key from the trie, returning
    /// `Some(val)` if a corresponding key/value pair was found.
    pub fn remove<L: Borrow<[u8]>>(&mut self, key: L) -> Option<V> {
        Node::remove(&mut self.root, key.borrow()).map(|leaf| leaf.val)
    }


    /// Remove all elements beginning with a given prefix from the trie, producing a subtrie.
    pub fn remove_prefix<L: Borrow<[u8]>>(&mut self, prefix: L) -> Trie<K, V> {
        Trie { root: Node::remove_prefix(&mut self.root, prefix.borrow()) }
    }


    /// Get the corresponding entry for the given key.
    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        match self.root {
            Some(..) => {
                let (exemplar_ptr, mismatch) = {
                    let node = unsafe { self.root.as_mut().unchecked_unwrap() };
                    let exemplar = node.get_exemplar_mut(key.borrow());
                    let mismatch = nybble_get_mismatch(exemplar.key_slice(), key.borrow());
                    (exemplar as *mut Leaf<K, V>, mismatch)
                };

                match mismatch {
                    None => Entry::Occupied(OccupiedEntry {
                        _dummy: PhantomData,

                        leaf: exemplar_ptr,
                        root: (&mut self.root) as *mut Option<Node<K, V>>,
                    }),

                    Some((b, i)) => {
                        println!("Entry API: {:?} vs {:?}: {:?}", unsafe { &*exemplar_ptr }.key_slice(), key.borrow(), (b, i));
                        // 11110
                        // 01101
                        let node = unsafe { self.root.as_mut().unchecked_unwrap() };

                        Entry::Vacant(VacantEntry {
                            key,
                            inner: VacantEntryInner::Internal(i, b, node),
                        })
                    }
                }
            }

            None => Entry::Vacant(VacantEntry {
                key,
                inner: VacantEntryInner::Root(&mut self.root),
            }),
        }
    }
}


impl<K: ToOwned + Borrow<[u8]>, V, L: Borrow<[u8]>> Index<L> for Trie<K, V> {
    type Output = V;

    fn index(&self, key: L) -> &V {
        self.get(key).unwrap()
    }
}


impl<K: ToOwned + Borrow<[u8]>, V, L: Borrow<[u8]>> IndexMut<L> for Trie<K, V> {
    fn index_mut(&mut self, key: L) -> &mut V {
        self.get_mut(key).unwrap()
    }
}


#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;

    use rand::Rng;
    use quickcheck::TestResult;

    quickcheck! {
        fn nybble(nybs: Vec<u8>) -> TestResult {
            for &nyb in &nybs {
                if nyb > 15 {
                    return TestResult::discard();
                }
            }

            let mut bytes = Vec::new();

            for chunk in nybs.chunks(2) {
                if chunk.len() == 2 {
                    bytes.push(chunk[0] | (chunk[1] << 4));
                } else {
                    bytes.push(chunk[0]);
                }
            }

            for (i, nyb) in nybs.into_iter().enumerate() {
                assert_eq!(nyb + 1, nybble_index(i, &bytes));
            }

            TestResult::passed()
        }

        fn insert_and_get(elts: Vec<(u8, u64)>) -> bool {
            let mut elts = elts;
            let mut rng = rand::thread_rng();
            elts.sort_by_key(|e| e.0);
            elts.dedup_by_key(|e| e.0);
            rng.shuffle(&mut elts);

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
                if trie.get([key]) != Some(&value) {
                    return false;
                }
            }

            for (&key, &value) in trie.iter() {
                if hashmap[&key[0]] != value {
                    return false;
                }
            }

            return true;
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
                        trie.remove(k.as_ref());
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

            filtered == prefixed
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

        println!("Trie: {:?}\nTrie root: {:?}", trie, trie.root);

        let filtered: HashMap<&[u8], u64> = trie.iter()
            .filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) {
                Some((key, val))
            } else {
                None
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
                    trie.remove(k.as_ref());
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
                println!("Key as bits: {} == {:b}", b, b);

                assert_eq!(trie.count(), i);
                trie.insert([b], s);

                println!("Trie: {:?}", trie);
                println!("Root: {:?}", trie.root);
            }

            trie
        };


        for (key, value) in hashmap {
            assert_eq!(
                trie.get([key]),
                Some(&value),
                "Sad trie: {:?}, sad root: {:?}",
                trie,
                trie.root
            );
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
}
