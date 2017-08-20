use trie::Trie;

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Visitor, MapAccess};
use serde::ser::{Serialize, Serializer, SerializeMap};


impl<K, V> Serialize for Trie<K, V>
where
    K: Serialize + Borrow<[u8]>,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {   
        let mut map = serializer.serialize_map(Some(self.count()))?;
        for (k, v) in self.iter() {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}


struct TrieVisitor<K, V> {
    marker: PhantomData<fn() -> Trie<K, V>>,
}


impl<K, V> TrieVisitor<K, V> {
    fn new() -> Self {
        TrieVisitor {
            marker: PhantomData,
        }
    }
}


impl<'de, K, V> Visitor<'de> for TrieVisitor<K, V>
where
    K: Deserialize<'de> + Borrow<[u8]>,
    V: Deserialize<'de>,
{
    type Value = Trie<K, V>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a qp-trie")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where M: MapAccess<'de>
    {
        let mut trie = Trie::new();
        while let Some((key, value)) = access.next_entry()? {
            trie.insert(key, value);
        }

        Ok(trie)
    }
}


impl<'de, K, V> Deserialize<'de> for Trie<K, V>
where
    K: Deserialize<'de> + Borrow<[u8]>,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TrieVisitor::new())
    }
}
