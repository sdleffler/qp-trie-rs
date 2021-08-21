use std::borrow::Borrow;
use std::borrow::Cow;

/// A trait for keys in a QP-trie.
///
/// Implementing types must be borrowable in the form of both a key slice,
/// such as `&str`, and the plain byte slice `&[u8]`. The former is used in
/// the public `trie::Trie` API, while the latter is used internally to match
/// and store keys.
///
/// Note that, as a consequence, keys which are not bytewise-equivalent will
/// not associate to the same entry, even if they are equal under `Eq`.
pub trait AsKey {
    /// The borrowed form of this key type.
    type Borrowed: ?Sized;

    /// View the key slice as a plain byte sequence.
    fn nybbles_from(key: &Self::Borrowed) -> &[u8];

    /// Borrow the key as nybbles, in the form of a plain byte sequence.
    fn as_nybbles(&self) -> &[u8];
}

macro_rules! impl_for_borrowables {
    ( $type:ty, $life:lifetime; $borrowed:ty; $view:ident ) => {
        impl<$life> AsKey for &$life $type  {
            type Borrowed = $borrowed;

            #[inline]
            fn as_nybbles(&self) -> &[u8] {
                self.$view()
            }

            #[inline]
            fn nybbles_from(key: &Self::Borrowed) -> &[u8] {
                key.$view()
            }
        }
     };
     ( $type:ty; $borrowed:ty; $view:ident ) => {
        impl AsKey for $type  {
            type Borrowed = $borrowed;

            #[inline]
            fn as_nybbles(&self) -> &[u8] {
                self.$view()
            }

            #[inline]
            fn nybbles_from(key: &Self::Borrowed) -> &[u8] {
                key.$view()
            }
        }
     }
}

impl_for_borrowables! { [u8], 'a; [u8]; as_ref }
impl_for_borrowables! { Vec<u8>; [u8]; as_ref }
impl_for_borrowables! { Cow<'a, [u8]>, 'a; [u8]; as_ref }

impl_for_borrowables! { str, 'a; str; as_bytes }
impl_for_borrowables! { String; str; as_bytes }
impl_for_borrowables! { Cow<'a, str>, 'a; str; as_bytes }

macro_rules! impl_for_arrays_of_size {
    ($($length:expr)+) => { $(
        impl AsKey for [u8; $length] {
            type Borrowed = [u8];

            #[inline]
            fn as_nybbles(&self) -> &[u8] {
                self.as_ref()
            }

            #[inline]
            fn nybbles_from(key: &Self::Borrowed) -> &[u8] {
                key
            }
        }
    )+ }
}

impl_for_arrays_of_size! {
    0  1  2  3  4  5  6  7  8  9
    10 11 12 13 14 15 16 17 18 19
    20 21 22 23 24 25 26 27 28 29
    30 31 32
}

/// Break!
pub trait Break<K: ?Sized>: AsKey {
    fn empty<'a>() -> &'a K;
    fn find_break(&self, loc: usize) -> &K;
    fn whole(&self) -> &K;
}

// All `AsKey`s can break as [u8], by construction of the qp-trie.
impl<'b, K> Break<[u8]> for K
where
    K: AsKey,
    K::Borrowed: Borrow<[u8]>,
{
    #[inline]
    fn empty<'a>() -> &'a [u8] {
        <&'a [u8]>::default()
    }

    #[inline]
    fn whole(&self) -> &[u8] {
        self.as_nybbles()
    }

    #[inline]
    fn find_break(&self, loc: usize) -> &[u8] {
        &self.as_nybbles()[..loc]
    }
}

impl<'b, K> Break<str> for K
where
    K: AsRef<str> + AsKey,
    K::Borrowed: Borrow<str>,
{
    #[inline]
    fn empty<'a>() -> &'a str {
        <&'a str>::default()
    }

    #[inline]
    fn whole(&self) -> &str {
        self.as_ref()
    }

    #[inline]
    fn find_break(&self, mut loc: usize) -> &str {
        let s: &str = self.as_ref();
        while !s.is_char_boundary(loc) {
            loc -= 1;
        }

        &s[..loc]
    }
}
