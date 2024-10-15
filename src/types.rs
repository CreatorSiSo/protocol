use std::{
    fmt::{Debug, Display},
    ops::{BitXor, Not},
};

#[derive(Clone, Copy)]
pub struct Byte(u8);

impl Byte {
    pub fn from_u8(data: u8) -> Self {
        Self(data)
    }

    pub fn upper_nibble(&self) -> Nibble {
        Nibble::from_u8(self.0 >> 4)
    }

    pub fn lower_nibble(&self) -> Nibble {
        Nibble::from_u8(self.0)
    }

    /// Whether all bits are set to one.
    pub fn all(&self) -> bool {
        self.0 == 0b11111111
    }
}

impl Not for Byte {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitXor for Byte {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0.bitxor(rhs.0))
    }
}

impl Display for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08b}", self.0)
    }
}

impl Debug for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Only the 4 lower bits are used, the 4 upper bits are ignored and undefined.
#[derive(Clone, Copy, Eq)]
pub struct Nibble(u8);

const NIBBLE_MASK: u8 = 0x0f;

impl Nibble {
    pub fn from_u8(data: u8) -> Self {
        Self(data)
    }

    pub fn get_lsb(&self, index: u8) -> bool {
        (self.0 >> index) & 0b0001 == 0b0001
    }

    pub fn get_msb(&self, index: u8) -> bool {
        (self.0 >> 3 - index) & 0b0001 == 0b0001
    }
}

impl PartialEq for Nibble {
    fn eq(&self, other: &Self) -> bool {
        self.0 & NIBBLE_MASK == other.0 & NIBBLE_MASK
    }
}

impl Not for Nibble {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitXor for Nibble {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0.bitxor(rhs.0))
    }
}

impl Display for Nibble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04b}", self.0 & 0x0f)
    }
}

impl Debug for Nibble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bool_to_u8 = |val| if val { 1u8 } else { 0u8 };
        write!(
            f,
            "[{}, {}, {}, {}]",
            bool_to_u8(self.get_msb(0)),
            bool_to_u8(self.get_msb(1)),
            bool_to_u8(self.get_msb(2)),
            bool_to_u8(self.get_msb(3))
        )
    }
}

#[test]
fn formatting() {
    assert_eq!(format!("{}", Nibble::from_u8(0b1111)), "1111");
    assert_eq!(format!("{}", Nibble::from_u8(0b0111)), "0111");
    assert_eq!(format!("{}", Nibble::from_u8(0b0000)), "0000");

    assert_eq!(format!("{}", Byte::from_u8(0b11110000)), "11110000");
    assert_eq!(format!("{}", Byte::from_u8(0b01010101)), "01010101");
    assert_eq!(format!("{}", Byte::from_u8(0b00000000)), "00000000");
}

#[test]
fn operations() {
    assert_eq!(
        Nibble::from_u8(0b0101).bitxor(Nibble::from_u8(0b0000)),
        Nibble::from_u8(0b0101)
    );

    assert_eq!(Nibble::from_u8(0b1000).get_msb(0), true);
    assert_eq!(Nibble::from_u8(0b1000).get_msb(1), false);
    assert_eq!(Nibble::from_u8(0b1000).get_msb(2), false);
    assert_eq!(Nibble::from_u8(0b1000).get_msb(3), false);

    assert_eq!(Nibble::from_u8(0b0010).get_lsb(0), false);
    assert_eq!(Nibble::from_u8(0b0010).get_lsb(1), true);
    assert_eq!(Nibble::from_u8(0b0010).get_lsb(2), false);
    assert_eq!(Nibble::from_u8(0b0010).get_lsb(3), false);
}
