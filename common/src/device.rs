use crate::bitvec::BitVec;

pub trait Device {
    /// Only reads upper nibble of byte.
    fn read(&mut self) -> BitVec<1>;

    /// Only sends upper nibble of byte.
    fn write(&mut self, data: BitVec<1>);

    /// TODO Remove, only used for debugging
    fn debug_poll(&mut self) {}
}
