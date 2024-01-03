use core::borrow::Borrow;
use core::fmt;
use core::ops::Index;

use crate::iter::Iter;
use crate::node::Node;

pub struct SubTrie<'a, K: 'a, V: 'a> {
    /// The index of the next byte to compare.
    key_byte_index: usize,
    root: Option<&'a Node<K, V>>,
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for SubTrie<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.root {
            Some(node) => f.debug_map().entries(node.iter()).finish(),
            None => f.debug_map().finish(),
        }
    }
}

impl<'a, K: 'a, V: 'a> IntoIterator for SubTrie<'a, K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.root.map(Node::iter).unwrap_or_default()
    }
}

impl<'a, K: 'a, V: 'a> SubTrie<'a, K, V> {
    pub fn new(root: Option<&'a Node<K, V>>, key_byte_index: usize) -> SubTrie<'a, K, V> {
        SubTrie {
            key_byte_index,
            root,
        }
    }

    pub fn empty() -> SubTrie<'a, K, V> {
        SubTrie::new(None, 0)
    }

    /// Returns true if the subtrie has no entries.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl<'a, K: Borrow<[u8]>, V> SubTrie<'a, K, V> {
    pub fn iter(&self) -> Iter<'a, K, V> {
        match self.root {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }

    pub fn iter_prefix<L: Borrow<[u8]>>(&self, prefix: L) -> Iter<'a, K, V> {
        match self.root.and_then(|node| node.get_prefix(prefix.borrow())) {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }

    /// Takes the next step in the trie, returning a new subtrie.
    pub fn subtrie<L: Borrow<[u8]>>(&self, next_key_part: L) -> SubTrie<'a, K, V> {
        let root = match self.root {
            Some(node) => node,
            None => return SubTrie::empty(),
        };
        let node = root.get_prefix_with_offset(next_key_part.borrow(), self.key_byte_index);
        SubTrie::new(node, self.key_byte_index + next_key_part.borrow().len())
    }

    /// Gets the value at the root of the subtrie.
    /// Only returns a value if we're at the end of the key.
    pub fn get_value(&self) -> Option<&'a V> {
        self.root
            .and_then(|node| match node {
                Node::Leaf(leaf) => Some(leaf),
                Node::Branch(v) => v.head_entry(),
            })
            .and_then(|leaf| {
                if self.key_byte_index == leaf.key.borrow().len() {
                    Some(&leaf.val)
                } else {
                    None
                }
            })
    }

    /// Gets a subtrie rooted at the given prefix.
    /// Is slightly less efficient than `subtrie`, since it re-compares the prefix.
    pub fn subtrie_with_prefix<L: Borrow<[u8]>>(&self, prefix: L) -> SubTrie<'a, K, V> {
        let root = match self.root {
            Some(node) => node,
            None => return SubTrie::empty(),
        };
        let node = root.get_prefix(prefix.borrow());
        SubTrie::new(node, prefix.borrow().len())
    }

    pub fn get<L: Borrow<[u8]>>(&self, key: L) -> Option<&'a V> {
        self.root
            .and_then(|node| node.get(key.borrow()))
            .map(|leaf| &leaf.val)
    }
}

impl<'a, K: Borrow<[u8]>, V, L: Borrow<[u8]>> Index<L> for SubTrie<'a, K, V> {
    type Output = V;

    fn index(&self, key: L) -> &V {
        self.get(key).unwrap()
    }
}
