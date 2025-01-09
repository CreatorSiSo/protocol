pub trait BitIter {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool>;
}

impl BitIter for bool {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool> {
        core::iter::once(*self)
    }
}

impl BitIter for &[bool] {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool> {
        self.into_iter().copied()
    }
}

impl<const N: usize> BitIter for [bool; N] {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool> {
        self.into_iter().copied()
    }
}

pub struct Byte(pub u8);

impl BitIter for Byte {
    fn iter(&self) -> impl ExactSizeIterator<Item = bool> {
        (0..8).rev().map(|i| (self.0 >> i) & 1 == 1)
    }
}

#[test]
fn byte_iter() {
    let mut iter = Byte(0xf0).iter();
    assert_eq!(iter.next(), Some(true));
    assert_eq!(iter.next(), Some(true));
    assert_eq!(iter.next(), Some(true));
    assert_eq!(iter.next(), Some(true));
    assert_eq!(iter.next(), Some(false));
    assert_eq!(iter.next(), Some(false));
    assert_eq!(iter.next(), Some(false));
    assert_eq!(iter.next(), Some(false));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}
