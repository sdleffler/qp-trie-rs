#[macro_use]
extern crate debug_unreachable;
extern crate unreachable;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[macro_use]
#[cfg(test)]
extern crate quickcheck;

#[cfg(feature = "serde")]
mod serialization;

mod entry;
mod iter;
mod key;
mod node;
mod sparse;
mod subtrie;
mod trie;
mod util;

pub mod wrapper;

pub use entry::{Entry, OccupiedEntry, VacantEntry};
pub use iter::{IntoIter, Iter, IterMut};
pub use key::{AsKey, Break};
pub use subtrie::SubTrie;
pub use trie::Trie;
