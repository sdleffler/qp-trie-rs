use alloc::borrow::ToOwned;
use alloc::string::String;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

use trie::Break;

/// A wrapper for `String` which implements `Borrow<[u8]>` and hashes in the same way as a byte
/// slice.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BString(String);

impl fmt::Debug for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<BString> for String {
    #[inline]
    fn from(bs: BString) -> String {
        bs.0
    }
}

impl From<String> for BString {
    #[inline]
    fn from(s: String) -> BString {
        BString(s)
    }
}

impl<'a> From<&'a str> for BString {
    #[inline]
    fn from(s: &'a str) -> BString {
        BString(s.into())
    }
}

impl Deref for BString {
    type Target = BStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        From::from(self.0.as_str())
    }
}

impl Borrow<BStr> for BString {
    #[inline]
    fn borrow(&self) -> &BStr {
        &*self
    }
}

impl Borrow<[u8]> for BString {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Hash for BString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state);
    }
}

impl Break for BString {
    type Split = BStr;

    #[inline]
    fn empty<'a>() -> &'a BStr {
        BStr::empty()
    }

    #[inline]
    fn find_break(&self, loc: usize) -> &BStr {
        (**self).find_break(loc)
    }
}

/// A wrapper type for `str` which implements `Borrow<[u8]>` and hashes in the same way as a byte
/// slice.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct BStr(str);

impl fmt::Debug for BStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> From<&'a str> for &'a BStr {
    fn from(s: &'a str) -> &'a BStr {
        unsafe { &*(s as *const str as *const BStr) }
    }
}

impl ToOwned for BStr {
    type Owned = BString;

    #[inline]
    fn to_owned(&self) -> BString {
        self.0.to_owned().into()
    }
}

impl Borrow<[u8]> for BStr {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Hash for BStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state);
    }
}

impl Break for BStr {
    type Split = BStr;

    #[inline]
    fn empty<'a>() -> &'a BStr {
        <&'a BStr>::from(<&'a str>::default())
    }

    #[inline]
    fn find_break(&self, mut loc: usize) -> &BStr {
        while !self.0.is_char_boundary(loc) {
            loc -= 1;
        }

        From::from(&self.as_str()[..loc])
    }
}

impl BStr {
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<BStr> for str {
    fn as_ref(&self) -> &BStr {
        <&BStr>::from(self)
    }
}
