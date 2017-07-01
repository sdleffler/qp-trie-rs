use std::borrow::Borrow;
use std::fmt;
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

use entry::{make_entry, Entry};
use iter::{Iter, IterMut, IntoIter};
use node::{Node, Leaf};

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
/// trie.insert(b"abbc", 1);
/// trie.insert(b"abcd", 2);
/// trie.insert(b"bcde", 3);
/// trie.insert(b"bdde", 4);
/// trie.insert(b"bddf", 5);
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
/// assert_eq!(trie.get(b"abcd"), Some(&2));
/// assert_eq!(trie.get(b"bcde"), Some(&3));
///
/// // We can take subtries, removing all elements of the trie with a given prefix.
/// let mut subtrie = trie.remove_prefix(b"b");
///
/// assert_eq!(trie.get(b"abbc"), Some(&1));
/// assert_eq!(trie.get(b"abcd"), Some(&2));
/// assert_eq!(trie.get(b"bcde"), None);
/// assert_eq!(trie.get(b"bdde"), None);
/// assert_eq!(trie.get(b"bddf"), None);
///
/// assert_eq!(subtrie.get(b"abbc"), None);
/// assert_eq!(subtrie.get(b"abcd"), None);
/// assert_eq!(subtrie.get(b"bcde"), Some(&3));
/// assert_eq!(subtrie.get(b"bdde"), Some(&4));
/// assert_eq!(subtrie.get(b"bddf"), Some(&5));
///
/// // We can remove elements:
/// assert_eq!(trie.remove(b"abbc"), Some(1));
/// assert_eq!(trie.get(b"abbc"), None);
///
/// // We can mutate values:
/// *subtrie.get_mut(b"bdde").unwrap() = 0;
/// assert_eq!(subtrie.get(b"bdde"), Some(&0));
/// ```
pub struct Trie<K: ToOwned, V> {
    root: Option<Node<K, V>>,
}


impl<K: ToOwned, V: Clone> Clone for Trie<K, V> {
    fn clone(&self) -> Self {
        Trie { root: self.root.clone() }
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


impl<K: ToOwned, V> IntoIterator for Trie<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K::Owned, V);

    fn into_iter(self) -> Self::IntoIter {
        self.root.map(Node::into_iter).unwrap_or_else(
            IntoIter::default,
        )
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


impl<K: ToOwned + Borrow<[u8]>, V> Extend<(K, V)> for Trie<K, V> {
    fn extend<I>(&mut self, iterable: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, val) in iterable {
            self.insert(key, val);
        }
    }
}


impl<K: ToOwned, V> Default for Trie<K, V> {
    fn default() -> Self {
        Trie { root: None }
    }
}


impl<K: ToOwned + Borrow<[u8]>, V: PartialEq> PartialEq for Trie<K, V> {
    fn eq(&self, rhs: &Trie<K, V>) -> bool {
        self.root == rhs.root
    }
}


impl<K: ToOwned + Borrow<[u8]>, V: Eq> Eq for Trie<K, V> {}


impl<K: ToOwned + Borrow<[u8]>, V> Trie<K, V> {
    /// Create a new, empty trie.
    pub fn new() -> Trie<K, V> {
        Trie { root: None }
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


    /// Iterate over all elements with a given prefix.
    pub fn iter_prefix<L: Borrow<[u8]>>(&self, prefix: L) -> Iter<K, V> {
        match self.root.as_ref().and_then(
            |node| node.get_prefix(prefix.borrow()),
        ) {
            Some(node) => Iter::new(node),
            None => Iter::default(),
        }
    }


    /// Iterate over all elements with a given prefix, but given a mutable reference to the
    /// associated value.
    pub fn iter_prefix_mut<L: Borrow<[u8]>>(&mut self, prefix: L) -> IterMut<K, V> {
        match self.root.as_mut().and_then(|node| {
            node.get_prefix_mut(prefix.borrow())
        }) {
            Some(node) => IterMut::new(node),
            None => IterMut::default(),
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
        make_entry(key, &mut self.root)
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
