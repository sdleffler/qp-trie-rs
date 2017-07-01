use std::cmp;

// Get the "nybble index" corresponding to the `n`th nybble in the given slice.
//
// This is `1 + b` where `b` is the `n`th nybble, unless the given slice has less than `n / 2`
// elements, in which case `0` is returned.
#[inline]
pub fn nybble_index(n: usize, slice: &[u8]) -> u8 {
    let byte_idx = n / 2;

    if byte_idx < slice.len() {
        let byte = slice[byte_idx];

        // If the index is even, return the lower nybble. Even, the higher nybble.
        // In both cases, increment by one. The zero-index is reserved for the "head" of the sparse
        // array.
        if n & 1 == 0 {
            1 + (byte & 0x0F)
        } else {
            1 + (byte >> 4)
        }
    } else {
        // If the nybble is out-of-range, we return zero. This is not some sort of weird
        // convention which would be clearer served by an `Option`; instead, we're actually
        // returning the "head" index of the sparse array. In the case that our trie `Branch` node
        // here - say it's branching at the `nth` nybble - contains a single entry of exactly `n /
        // 2` bytes long, then we have to have someplace to put it - the head. Essentially the head
        // is where leaf nodes which do not live at the fringes of the tree are stored.
        0
    }
}


// Find the nybble at which the two provided slices mismatch. If no such nybble exists and the
// slices are the same length, `None` is returned; if no such nybble exists but the slices are
// *not* the same length, then the point at which one slice has a byte and the other has ended is
// considered the mismatch point.
#[inline]
pub fn nybble_mismatch(left: &[u8], right: &[u8]) -> Option<usize> {
    let mut difference;

    for (i, (l, r)) in left.iter().cloned().zip(right.iter().cloned()).enumerate() {
        difference = l ^ r;

        if difference != 0 {
            if difference & 0x0F == 0 {
                return Some(1 + i * 2);
            } else {
                return Some(i * 2);
            }
        }
    }

    if left.len() == right.len() {
        None
    } else {
        Some(cmp::min(left.len(), right.len()) * 2)
    }
}


#[inline]
pub fn nybble_get_mismatch(left: &[u8], right: &[u8]) -> Option<(u8, usize)> {
    let mut difference;

    for (i, (l, r)) in left.iter().cloned().zip(right.iter().cloned()).enumerate() {
        difference = l ^ r;

        if difference != 0 {
            if difference & 0x0F == 0 {
                return Some((1 + (l >> 4), 1 + i * 2));
            } else {
                return Some((1 + (l & 0x0F), i * 2));
            }
        }
    }

    if left.len() == right.len() {
        None
    } else {
        let idx = cmp::min(left.len(), right.len()) * 2;

        Some((nybble_index(idx, left), idx))
    }
}



