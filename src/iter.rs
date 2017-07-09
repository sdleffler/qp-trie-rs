use std::borrow::Borrow;

use node::Node;


/// An iterator over the keys and values in a QP-trie.
pub struct IntoIter<K: ToOwned, V> {
    stack: Vec<Node<K, V>>,
}


impl<K: ToOwned, V> IntoIter<K, V> {
    pub(crate) fn new(node: Node<K, V>) -> IntoIter<K, V> {
        IntoIter { stack: vec![node] }
    }
}


impl<K: ToOwned, V> Default for IntoIter<K, V> {
    fn default() -> Self {
        IntoIter { stack: vec![] }
    }
}


impl<K: ToOwned, V> Iterator for IntoIter<K, V> {
    type Item = (K::Owned, V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(Node::Leaf(leaf)) => Some((leaf.key, leaf.val)),
            Some(Node::Branch(branch)) => {
                self.stack.extend(branch.into_iter().rev());
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
    pub fn new(node: &'a Node<K, V>) -> Iter<'a, K, V> {
        Iter { stack: vec![node] }
    }
}

impl<'a, K: ToOwned, V> Default for Iter<'a, K, V> {
    fn default() -> Self {
        Iter { stack: vec![] }
    }
}


impl<'a, K: 'a + ToOwned, V: 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(&Node::Leaf(ref leaf)) => Some((leaf.key.borrow(), &leaf.val)),
            Some(&Node::Branch(ref branch)) => {
                self.stack.extend(branch.iter().rev());
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
    pub fn new(node: &'a mut Node<K, V>) -> IterMut<'a, K, V> {
        IterMut { stack: vec![node] }
    }
}


impl<'a, K: ToOwned, V> Default for IterMut<'a, K, V> {
    fn default() -> Self {
        IterMut { stack: vec![] }
    }
}


impl<'a, K: 'a + ToOwned, V: 'a> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            Some(&mut Node::Leaf(ref mut leaf)) => Some((leaf.key.borrow(), &mut leaf.val)),
            Some(&mut Node::Branch(ref mut branch)) => {
                self.stack.extend(branch.iter_mut().rev());
                self.next()
            }
            None => None,
        }
    }
}
