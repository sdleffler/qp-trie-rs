use std::borrow::Borrow;
use std::fmt;
use std::ops::Index;

use iter::Iter;
use node::Node;


pub struct SubTrie<'a, K: 'a + ToOwned, V: 'a> {
    pub(crate) root: Option<&'a Node<K, V>>,
}


impl<'a, K: fmt::Debug + ToOwned, V: fmt::Debug> fmt::Debug for SubTrie<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.root {
            Some(node) => f.debug_map().entries(node.iter()).finish(),
            None => f.debug_map().finish(),
        }
    }
}


impl<'a, K: 'a + ToOwned, V: 'a> IntoIterator for SubTrie<'a, K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.root.map(Node::iter).unwrap_or_default()
    }
}


impl<'a, K: ToOwned + Borrow<[u8]>, V> SubTrie<'a, K, V> {
    pub fn iter(&self) -> Iter<K, V> {
        match self.root {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }


    pub fn iter_prefix<L: Borrow<[u8]>>(&self, prefix: L) -> Iter<K, V> {
        match self.root.and_then(|node| node.get_prefix(prefix.borrow())) {
            Some(node) => node.iter(),
            None => Iter::default(),
        }
    }


    pub fn subtrie<L: Borrow<[u8]>>(&self, prefix: L) -> SubTrie<K, V> {
        SubTrie { root: self.root.and_then(|node| node.get_prefix(prefix.borrow())) }
    }


    pub fn get<L: Borrow<[u8]>>(&self, key: L) -> Option<&V> {
        self.root.and_then(|node| node.get(key.borrow())).map(
            |leaf| {
                &leaf.val
            },
        )
    }
}


impl<'a, K: ToOwned + Borrow<[u8]>, V, L: Borrow<[u8]>> Index<L> for SubTrie<'a, K, V> {
    type Output = V;

    fn index(&self, key: L) -> &V {
        self.get(key).unwrap()
    }
}
