use std::fmt;
use std::ops::Index;

use iter::Iter;
use key::AsKey;
use node::Node;

pub struct SubTrie<'a, K: 'a, V: 'a> {
    pub(crate) root: Option<&'a Node<K, V>>,
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
    /// Returns true if the subtrie has no entries.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl<'a, K: AsKey, V> SubTrie<'a, K, V> {
    pub fn iter(&self) -> Iter<K, V> {
        match self.root {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }

    pub fn iter_prefix<L: AsKey>(&self, prefix: L) -> Iter<K, V> {
        match self
            .root
            .and_then(|node| node.get_prefix(prefix.as_nybbles()))
        {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }

    pub fn subtrie<L: AsKey>(&self, prefix: L) -> SubTrie<K, V> {
        SubTrie {
            root: self
                .root
                .and_then(|node| node.get_prefix(prefix.as_nybbles())),
        }
    }

    pub fn get<L: AsKey>(&self, key: L) -> Option<&V> {
        self.root
            .and_then(|node| node.get(key.as_nybbles()))
            .map(|leaf| &leaf.val)
    }
}

impl<'a, K: AsKey, V, L: AsKey> Index<L> for SubTrie<'a, K, V> {
    type Output = V;

    fn index(&self, key: L) -> &V {
        self.get(key).unwrap()
    }
}
