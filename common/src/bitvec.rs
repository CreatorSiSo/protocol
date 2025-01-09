use core::fmt::Debug;
use core::fmt::Display;
use core::ops::BitOr;
use core::ops::Not;

use crate::BitIter;

// u8 array based bit vec with a maximum capacity set at compile time.
// The bit order is most significant bit at index 0.
#[derive(Eq, Clone, Copy)]
pub struct BitVec<const C: usize> {
    len: usize,
    bytes: [u8; C],
}

impl<const C: usize> BitVec<C> {
    pub fn new() -> Self {
        Self {
            len: 0,
            bytes: [0; C],
        }
    }

    pub fn from_bytes(bytes: [u8; C], len: usize) -> Self {
        let mut result = Self { len, bytes };
        result.clear_remaining_bits();
        result
    }

    pub fn len(&self) -> usize {
        self.len
    }

    // Push another BitVec to the front
    // Returns true if the BitVec has reached its capacity
    pub fn push_front(&mut self, other: &impl BitIter) {
        let iter = other.iter();
        // Ensure there is enough capacity to add the other BitVec
        if self.len + iter.len() > (C * 8) {
            panic!()
        }

        self.len += iter.len();
        self.shift_right(iter.len() as u32);

        // Insert the bits of the other BitVec at the front
        for (i, bit) in iter.enumerate() {
            self.set_unchecked(i, bit);
        }
    }

    pub fn push_back(&mut self, other: &impl BitIter) {
        let iter = other.iter();
        let iter_len = iter.len();

        // Ensure there is enough capacity to add the other BitVec
        if self.len + iter.len() > (C * 8) {
            panic!("Not enough capacity");
        }

        for (bit, i) in iter.zip(self.len..) {
            self.set_unchecked(i, bit);
        }

        self.len += iter_len;
    }

    pub fn pop_front<const R: usize>(&mut self, width: usize) -> Option<BitVec<R>> {
        debug_assert!(width <= (R * 8));

        if width > self.len {
            return None;
        }

        let mut result = BitVec {
            bytes: [0; R],
            len: width,
        };
        result.bytes.copy_from_slice(&self.bytes[..R]);
        result.clear_remaining_bits();

        self.shift_left(width as u32);
        self.len -= width;
        self.clear_remaining_bits();

        Some(result)
    }

    // Pop a number of bits from the back
    pub fn pop_back<const R: usize>(&mut self, width: usize) -> Option<BitVec<R>> {
        debug_assert!(width <= (R * 8));

        if width > self.len {
            return None;
        }

        let old_len = self.len;
        let new_len = old_len - width;

        let mut popped = BitVec {
            len: width,
            bytes: [0; R],
        };

        // Copy the bits that are being popped from self to popped
        let mut bit_index = 0;
        for i in new_len..old_len {
            let bit = (self.bytes[i / 8] >> (7 - i % 8)) & 1;
            popped.set_unchecked(bit_index, bit == 1);
            bit_index += 1;
        }

        self.len = new_len;
        self.clear_remaining_bits();

        Some(popped)
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        (index < self.len).then(|| {
            let byte = self.bytes[index / 8];
            // Reverse the bit index within the byte
            let mask = 1 << (7 - (index % 8));
            byte & mask != 0
        })
    }

    pub fn set(&mut self, index: usize, value: bool) {
        if index > self.len {
            panic!(
                "Index {index} out of range for BitVec with length {}!",
                self.len
            );
        }
        self.set_unchecked(index, value);
    }

    pub fn toggle(&mut self, index: usize) {
        if self.get(index).unwrap() == false {
            self.set_unchecked(index, true);
        } else {
            self.set_unchecked(index, false);
        }
    }

    fn set_unchecked(&mut self, index: usize, value: bool) {
        let byte_index = index / 8;
        let bit_index = 7 - (index % 8); // MSB at index 0

        if value {
            self.bytes[byte_index] |= 1 << bit_index;
        } else {
            self.bytes[byte_index] &= !(1 << bit_index);
        }
    }

    pub fn shift_left(&mut self, by: u32) {
        slice_shift_left(&mut self.bytes, by);
    }

    pub fn shift_right(&mut self, by: u32) {
        slice_shift_right(&mut self.bytes, by);
        self.clear_remaining_bits();
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn clear_remaining_bits(&mut self) {
        // Calculate the number of valid bytes and bits
        let valid_bytes = self.len / 8;
        let remaining_bits = self.len % 8;

        // Zero out trailing bytes
        for byte in self
            .bytes
            .iter_mut()
            .skip(valid_bytes + (remaining_bits > 0) as usize)
        {
            *byte = 0;
        }

        // Zero out trailing bits in the last valid byte
        if remaining_bits > 0 {
            let mask = 0xFF << (8 - remaining_bits); // Mask to keep only the valid bits
            self.bytes[valid_bytes] &= mask;
        }
    }
}

impl<const C: usize, B: BitIter> From<&B> for BitVec<C> {
    fn from(value: &B) -> Self {
        let mut result = Self::new();
        let iter = value.iter();
        result.len = iter.len();
        for (i, bit) in iter.enumerate() {
            result.set_unchecked(i, bit);
        }
        result
    }
}

impl<const C: usize> BitIter for BitVec<C> {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool> {
        (0..self.len).map(|i| self.get(i).unwrap())
    }
}

impl<const C: usize, const O: usize> PartialEq<BitVec<O>> for BitVec<C> {
    fn eq(&self, other: &BitVec<O>) -> bool {
        if self.len != other.len {
            return false;
        }

        // ignore data for empty bitvecs
        if self.len == 0 {
            return true;
        }

        // whether all filled bytes are the same
        let filled_up_to = self.len / 8;
        if self.bytes[..filled_up_to] != other.bytes[..filled_up_to] {
            return false;
        }

        let remaining_bits = self.len % 8;
        if remaining_bits > 0 {
            let mask = !(0xff >> remaining_bits);
            return self.bytes[filled_up_to] & mask == other.bytes[filled_up_to] & mask;
        }

        true
    }
}

impl<const C: usize> Not for BitVec<C> {
    type Output = BitVec<C>;

    fn not(mut self) -> Self::Output {
        for byte in self.bytes.iter_mut() {
            *byte = !*byte;
        }
        self.clear_remaining_bits();
        self
    }
}

impl<const C: usize> BitOr for BitVec<C> {
    type Output = BitVec<C>;

    fn bitor(self, rhs: Self) -> Self::Output {
        debug_assert!(self.len == rhs.len);
        let mut bytes = [0; C];
        bytes.copy_from_slice(&self.bytes);
        for i in 0..self.len {
            bytes[i] |= rhs.bytes[i];
        }
        Self {
            bytes,
            len: self.len,
        }
    }
}

impl<const C: usize> Display for BitVec<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut bits = self.iter().map(|bit| if bit { 1 } else { 0 });

        write!(f, "BitVec<{C}>[")?;
        if let Some(bit) = bits.next() {
            write!(f, "{bit}")?;
        }
        for bit in bits {
            write!(f, ", {bit}")?;
        }
        write!(f, "]")
    }
}

impl<const C: usize> Debug for BitVec<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

#[test]
fn bitvec_get() {
    let bitvec: BitVec<2> =
        BitVec::from(&[true, false, true, false, true, true, true, false, true]);
    assert_eq!(bitvec.get(0), Some(true));
    assert_eq!(bitvec.get(1), Some(false));
    assert_eq!(bitvec.get(7), Some(false));
    assert_eq!(bitvec.get(8), Some(true));
    assert_eq!(bitvec.get(9), None);

    assert_eq!(BitVec::from_bytes([0b1010_0000], 4).get(3), Some(false));
}

#[test]
fn bitvec_set() {
    let mut bitvec = BitVec::from_bytes([0b1010_1110, 0], 9);

    bitvec.set(0, false);
    assert_eq!(bitvec, BitVec::from_bytes([0b0010_1110, 0], 9));

    bitvec.set(2, false);
    bitvec.set(4, false);
    bitvec.set(5, false);
    bitvec.set(6, false);
    bitvec.set(7, false);
    bitvec.set(8, true);

    assert_eq!(bitvec, BitVec::from_bytes([0, 0b1000_0000], 9));
}

#[test]
fn bitvec_eq() {
    assert_eq!(
        BitVec::from_bytes([0b1010_0000], 0),
        BitVec::from_bytes([0b1010_1111], 0)
    );
    assert_eq!(
        BitVec::from_bytes([0b1010_0000], 3),
        BitVec::from_bytes([0b1010_1111], 3)
    );
    assert_eq!(
        BitVec::from_bytes([0, 0, 0], 24),
        BitVec::from_bytes([0, 0, 0], 24)
    );

    assert_ne!(BitVec::from_bytes([0], 0), BitVec::from_bytes([0], 1));
    assert_ne!(
        dbg!(BitVec::from_bytes([0b1010_0000], 4)),
        BitVec::from_bytes([0b1011_1111], 4)
    );
    assert_ne!(BitVec::from_bytes([0], 8), BitVec::from_bytes([1], 8));
}

#[test]
fn bitvec_shift() {
    let mut bitvec = BitVec::from_bytes([0, 0b0000_1000], 13);

    bitvec.shift_left(4);
    assert_eq!(bitvec, BitVec::from_bytes([0, 0b1000_0000], 13));

    bitvec.shift_right(2);
    assert_eq!(bitvec, BitVec::from_bytes([0, 0b0010_0000], 13));

    bitvec.shift_right(3);
    assert_eq!(bitvec, BitVec::from_bytes([0, 0b0000_0100], 13));
    assert_eq!(bitvec, BitVec::from_bytes([0, 0b0000_0000], 13));

    bitvec.shift_left(8);
    assert_eq!(bitvec, BitVec::from_bytes([0, 0], 13));
}

pub fn slice_shift_left(slice: &mut [u8], by: u32) {
    let len = slice.len();
    if len == 0 || by == 0 {
        return; // Nothing to do
    }

    // Calculate the full-byte shift and the bit shift
    let byte_shift = (by / 8) as usize; // Number of bytes to shift
    let bit_shift = (by % 8) as u8; // Remaining bits to shift

    if byte_shift >= len {
        // If the shift exceeds or equals the slice length, zero out the entire slice
        slice.fill(0);
        return;
    }

    // Shift full bytes to the left
    for i in 0..(len - byte_shift) {
        slice[i] = slice[i + byte_shift];
    }

    // Zero out the bytes shifted past the end
    for i in (len - byte_shift)..len {
        slice[i] = 0;
    }

    if bit_shift > 0 {
        // Handle the bit-level shift
        for i in 0..(len - 1) {
            slice[i] = (slice[i] << bit_shift) | (slice[i + 1] >> (8 - bit_shift));
        }
        slice[len - 1] <<= bit_shift; // Shift the last byte
    }
}

fn slice_shift_right(slice: &mut [u8], by: u32) {
    let len = slice.len();
    if len == 0 || by == 0 {
        return; // Nothing to do
    }

    // Calculate the full-byte shift and the bit shift
    let byte_shift = (by / 8) as usize; // Number of bytes to shift
    let bit_shift = (by % 8) as u8; // Remaining bits to shift

    if byte_shift >= len {
        // If the shift exceeds or equals the slice length, zero out the entire slice
        slice.fill(0);
        return;
    }

    // Shift full bytes to the right
    for i in (byte_shift..len).rev() {
        slice[i] = slice[i - byte_shift];
    }

    // Zero out the bytes shifted past the start
    for i in 0..byte_shift {
        slice[i] = 0;
    }

    if bit_shift > 0 {
        // Handle the bit-level shift
        for i in (1..len).rev() {
            slice[i] = (slice[i] >> bit_shift) | (slice[i - 1] << (8 - bit_shift));
        }
        slice[0] >>= bit_shift; // Shift the first byte
    }
}

#[test]
fn bit_shift_slices() {
    let mut array = [0b11111111, 0b11111111, 0, 0, 0, 0, 0, 0];
    slice_shift_left(&mut array, 3);
    assert_eq!(array, [0b11111111, 0b11111000, 0, 0, 0, 0, 0, 0]);
    slice_shift_left(&mut array, 8);
    assert_eq!(array, [0b11111000, 0, 0, 0, 0, 0, 0, 0]);
    slice_shift_left(&mut array, 300);
    assert_eq!(array, [0, 0, 0, 0, 0, 0, 0, 0]);

    array = [0, 0, 0, 0, 0, 0, 0b11111111, 0b11111111];
    slice_shift_right(&mut array, 3);
    assert_eq!(array, [0, 0, 0, 0, 0, 0, 0b00011111, 0b11111111]);
    slice_shift_right(&mut array, 10);
    assert_eq!(array, [0, 0, 0, 0, 0, 0, 0, 0b00000111]);
}
