use std::borrow::Borrow;
use std::fmt;
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

use entry::{make_entry, Entry};
use iter::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
use node::{Leaf, Node};
use subtrie::SubTrie;
use util::nybble_mismatch;
use wrapper::{BStr, BString};

/// A QP-trie. QP stands for - depending on who you ask - either "quelques-bits popcount" or
/// "quad-bit popcount". In any case, the fact of the matter is that this is a compressed radix
/// trie with a branching factor of 16. It acts as a key-value map where the keys are any value
/// which can be converted to a slice of bytes.
///
/// The following example uses the provided string wrapper. Unfortunately, `String`/`str` cannot be
/// used directly because they do not implement `Borrow<[u8]>` (as they do not hash the same way as
/// a byte slice.) As a stopgap, `qp_trie::wrapper::{BString, BStr}` are provided, as are the
/// `.whatever_str()` convenience methods on `qp_trie::Trie<BString, _>`.
///
/// # Example
///
/// ```rust
/// # use qp_trie::Trie;
///
/// let mut trie = Trie::new();
///
/// trie.insert_str("abbc", 1);
/// trie.insert_str("abcd", 2);
/// trie.insert_str("bcde", 3);
/// trie.insert_str("bdde", 4);
/// trie.insert_str("bddf", 5);
///
/// // This will print the following string:
/// //
/// // `{"abbc": 1, "abcd": 2, "bcde": 3, "bdde": 4, "bddf": 5}`
///
/// println!("{:?}", trie);
/// # assert_eq!(format!("{:?}", trie), "{\"abbc\": 1, \"abcd\": 2, \"bcde\": 3, \"bdde\": 4, \"bddf\": 5}");
///
/// assert_eq!(trie.get_str("abcd"), Some(&2));
/// assert_eq!(trie.get_str("bcde"), Some(&3));
///
/// // We can take subtries, removing all elements of the trie with a given prefix.
/// let mut subtrie = trie.remove_prefix_str("b");
///
/// assert_eq!(trie.get_str("abbc"), Some(&1));
/// assert_eq!(trie.get_str("abcd"), Some(&2));
/// assert_eq!(trie.get_str("bcde"), None);
/// assert_eq!(trie.get_str("bdde"), None);
/// assert_eq!(trie.get_str("bddf"), None);
///
/// assert_eq!(subtrie.get_str("abbc"), None);
/// assert_eq!(subtrie.get_str("abcd"), None);
/// assert_eq!(subtrie.get_str("bcde"), Some(&3));
/// assert_eq!(subtrie.get_str("bdde"), Some(&4));
/// assert_eq!(subtrie.get_str("bddf"), Some(&5));
///
/// // We can remove elements:
/// assert_eq!(trie.remove_str("abbc"), Some(1));
/// assert_eq!(trie.get_str("abbc"), None);
///
/// // We can mutate values:
/// *subtrie.get_mut_str("bdde").unwrap() = 0;
/// assert_eq!(subtrie.get_str("bdde"), Some(&0));
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Trie<K, V> {
    root: Option<Node<K, V>>,
    count: usize,
}

impl<K, V> Default for Trie<K, V> {
    fn default() -> Self {
        Trie::new()
    }
}

impl<K: fmt::Debug + ToOwned, V: fmt::Debug> fmt::Debug for Trie<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.root {
            Some(ref node) => f.debug_map().entries(node.iter()).finish(),
            None => f.debug_map().finish(),
        }
    }
}

impl<K, V> IntoIterator for Trie<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K, V);

    fn into_iter(self) -> Self::IntoIter {
        self.root
            .map(Node::into_iter)
            .unwrap_or_else(IntoIter::default)
    }
}

impl<K: Borrow<[u8]>, V> FromIterator<(K, V)> for Trie<K, V> {
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

impl<K: Borrow<[u8]>, V> Extend<(K, V)> for Trie<K, V> {
    fn extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, val) in iterable {
            self.insert(key, val);
        }
    }
}

impl<K, V> Trie<K, V> {
    /// Create a new, empty trie.
    pub fn new() -> Trie<K, V> {
        Trie {
            root: None,
            count: 0,
        }
    }

    /// Iterate over all elements in the trie.
    pub fn iter(&self) -> Iter<K, V> {
        match self.root {
            Some(ref node) => Iter::new(node),
            None => Iter::default(),
        }
    }

    /// Iterate over all elements in the trie, given a mutable reference to the associated value.
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        match self.root {
            Some(ref mut node) => IterMut::new(node),
            None => IterMut::default(),
        }
    }

    /// Iterate over all keys in the trie.
    pub fn keys(&self) -> Keys<K, V> {
        match self.root {
            Some(ref node) => Keys::new(node),
            None => Keys::default(),
        }
    }

    /// Iterate over all values in the trie.
    pub fn values(&self) -> Values<K, V> {
        match self.root {
            Some(ref node) => Values::new(node),
            None => Values::default(),
        }
    }

    /// Iterate over all values in the trie, mutably.
    pub fn values_mut(&mut self) -> ValuesMut<K, V> {
        match self.root {
            Some(ref mut node) => ValuesMut::new(node),
            None => ValuesMut::default(),
        }
    }

    /// Remove all entries from the trie, leaving it empty.
    pub fn clear(&mut self) {
        self.root = None;
    }

    /// Returns true if the trie has no entries.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl<K: Borrow<[u8]>, V> Trie<K, V> {
    /// Iterate over all elements with a given prefix.
    pub fn iter_prefix<'a, Q: ?Sized>(&self, prefix: &'a Q) -> Iter<K, V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        match self
            .root
            .as_ref()
            .and_then(|node| node.get_prefix(prefix.borrow()))
        {
            Some(node) => Iter::new(node),
            None => Iter::default(),
        }
    }

    /// Iterate over all elements with a given prefix, but given a mutable reference to the
    /// associated value.
    pub fn iter_prefix_mut<'a, Q: ?Sized>(&mut self, prefix: &'a Q) -> IterMut<K, V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        match self
            .root
            .as_mut()
            .and_then(|node| node.get_prefix_mut(prefix.borrow()))
        {
            Some(node) => IterMut::new(node),
            None => IterMut::default(),
        }
    }

    /// Get an immutable view into the trie, providing only values keyed with the given prefix.
    pub fn subtrie<'a, Q: ?Sized>(&self, prefix: &'a Q) -> SubTrie<K, V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        SubTrie {
            root: self
                .root
                .as_ref()
                .and_then(|node| node.get_prefix(prefix.borrow())),
        }
    }

    /// Get the longest common prefix of all the nodes in the trie and the given key.
    pub fn longest_common_prefix<'a, Q: ?Sized>(&self, key: &'a Q) -> &K::Split
    where
        K: Borrow<Q> + Break,
        Q: Borrow<[u8]>,
    {
        match self.root.as_ref() {
            Some(root) => {
                let exemplar = root.get_exemplar(key.borrow());

                match nybble_mismatch(exemplar.key_slice(), key.borrow()) {
                    Some(i) => exemplar.key.find_break(i / 2),
                    None => exemplar.key.borrow(),
                }
            }
            None => K::empty(),
        }
    }

    /// Count the number of entries in the tree.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns true if there is an entry for the given key.
    pub fn contains_key<'a, Q: ?Sized>(&self, key: &'a Q) -> bool
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        self.root
            .as_ref()
            .and_then(|node| node.get(key.borrow()))
            .is_some()
    }

    /// Get an immutable reference to the value associated with a given key, if it is in the tree.
    pub fn get<'a, Q: ?Sized>(&self, key: &'a Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        self.root
            .as_ref()
            .and_then(|node| node.get(key.borrow()))
            .map(|leaf| &leaf.val)
    }

    /// Get a mutable reference to the value associated with a given key, if it is in the tree.
    pub fn get_mut<'a, Q: ?Sized>(&mut self, key: &'a Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        self.root
            .as_mut()
            .and_then(|node| node.get_mut(key.borrow()))
            .map(|leaf| &mut leaf.val)
    }

    /// Insert a key/value pair into the trie, returning the old value if an entry already existed.
    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        match self.root {
            Some(ref mut root) => {
                let old = root.insert(key, val);
                if old.is_none() {
                    self.count += 1;
                }
                old
            }
            None => {
                self.root = Some(Node::Leaf(Leaf::new(key, val)));
                self.count += 1;
                None
            }
        }
    }

    /// Remove the key/value pair associated with a given key from the trie, returning
    /// `Some(val)` if a corresponding key/value pair was found.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        let node = Node::remove(&mut self.root, key.borrow()).map(|leaf| leaf.val);
        if node.is_some() {
            self.count -= 1;
        }
        node
    }

    /// Remove all elements beginning with a given prefix from the trie, producing a subtrie.
    pub fn remove_prefix<'a, Q: ?Sized>(&mut self, prefix: &'a Q) -> Trie<K, V>
    where
        K: Borrow<Q>,
        Q: Borrow<[u8]>,
    {
        let root = Node::remove_prefix(&mut self.root, prefix.borrow());
        let count = root.as_ref().map(Node::count).unwrap_or(0);
        self.count -= count;
        Trie { root, count }
    }

    /// Get the corresponding entry for the given key.
    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        make_entry(key, &mut self.root)
    }
}

impl<'a, K: Borrow<[u8]>, V, Q: ?Sized> Index<&'a Q> for Trie<K, V>
where
    K: Borrow<Q>,
    Q: Borrow<[u8]>,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).unwrap()
    }
}

impl<'a, K: Borrow<[u8]>, V, Q: ?Sized> IndexMut<&'a Q> for Trie<K, V>
where
    K: Borrow<Q>,
    Q: Borrow<[u8]>,
{
    fn index_mut(&mut self, key: &Q) -> &mut V {
        self.get_mut(key).unwrap()
    }
}

pub trait Break: Borrow<<Self as Break>::Split> {
    type Split: ?Sized;

    fn empty<'a>() -> &'a Self::Split;
    fn find_break(&self, loc: usize) -> &Self::Split;
}

impl Break for [u8] {
    type Split = [u8];

    #[inline]
    fn empty<'a>() -> &'a [u8] {
        <&'a [u8]>::default()
    }

    #[inline]
    fn find_break(&self, loc: usize) -> &[u8] {
        &self[..loc]
    }
}

impl<'b> Break for &'b [u8] {
    type Split = [u8];

    #[inline]
    fn empty<'a>() -> &'a [u8] {
        <&'a [u8]>::default()
    }

    #[inline]
    fn find_break(&self, loc: usize) -> &[u8] {
        &self[..loc]
    }
}

impl<V> Trie<BString, V> {
    /// Convenience function for iterating over suffixes with a string.
    pub fn iter_prefix_str<'a, Q: ?Sized>(&self, key: &'a Q) -> Iter<BString, V>
    where
        Q: Borrow<str>,
    {
        self.iter_prefix(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for iterating over suffixes with a string.
    pub fn iter_prefix_mut_str<'a, Q: ?Sized>(&mut self, key: &'a Q) -> IterMut<BString, V>
    where
        Q: Borrow<str>,
    {
        self.iter_prefix_mut(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for viewing subtries wit a string prefix.
    pub fn subtrie_str<'a, Q: ?Sized>(&self, prefix: &'a Q) -> SubTrie<BString, V>
    where
        Q: Borrow<str>,
    {
        self.subtrie(AsRef::<BStr>::as_ref(prefix.borrow()))
    }

    /// Returns true if there is an entry for the given string key.
    pub fn contains_key_str<'a, Q: ?Sized>(&self, key: &'a Q) -> bool
    where
        Q: Borrow<str>,
    {
        self.contains_key(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for getting with a string.
    pub fn get_str<'a, Q: ?Sized>(&self, key: &'a Q) -> Option<&V>
    where
        Q: Borrow<str>,
    {
        self.get(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for getting mutably with a string.
    pub fn get_mut_str<'a, Q: ?Sized>(&mut self, key: &'a Q) -> Option<&mut V>
    where
        Q: Borrow<str>,
    {
        self.get_mut(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for inserting with a string.
    pub fn insert_str<'a, Q: ?Sized>(&mut self, key: &'a Q, val: V) -> Option<V>
    where
        Q: Borrow<str>,
    {
        self.insert(key.borrow().into(), val)
    }

    /// Convenience function for removing with a string.
    pub fn remove_str<'a, Q: ?Sized>(&mut self, key: &'a Q) -> Option<V>
    where
        Q: Borrow<str>,
    {
        self.remove(AsRef::<BStr>::as_ref(key.borrow()))
    }

    /// Convenience function for removing a prefix with a string.
    pub fn remove_prefix_str<'a, Q: ?Sized>(&mut self, prefix: &'a Q) -> Trie<BString, V>
    where
        Q: Borrow<str>,
    {
        self.remove_prefix(AsRef::<BStr>::as_ref(prefix.borrow()))
    }
}
