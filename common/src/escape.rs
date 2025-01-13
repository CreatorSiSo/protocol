use crate::{bititer::Byte, BitVec};

#[derive(Debug, ufmt::derive::uDebug, PartialEq, Eq)]
#[repr(u8)]
pub enum EscapeCode {
    // SRQ
    SyncReq = 0b1110_0001,
    // SRS
    SyncRes = 0b1000_0110,
    /// SOF
    StartOfFrame = 0b1010_0101,
    /// ACK
    Ack = 0b1001_1001,
    /// NCK
    Nack = 0b1101_0001,
    // FS
    Finished = 0b1011_0001,
    // NOP
    Noop = 0b0000_0000,
}

impl EscapeCode {
    const VALUES: [(EscapeCode, u8); 7] = [
        (Self::StartOfFrame, Self::StartOfFrame as u8),
        (Self::Ack, Self::Ack as u8),
        (Self::Nack, Self::Nack as u8),
        (Self::SyncReq, Self::SyncReq as u8),
        (Self::SyncRes, Self::SyncRes as u8),
        (Self::Finished, Self::Finished as u8),
        (Self::Noop, Self::Noop as u8),
    ];

    pub fn all() -> impl Iterator<Item = (Self, BitVec<1>)> {
        Self::VALUES
            .into_iter()
            .map(|(code, byte)| (code, BitVec::from(Byte(byte))))
    }

    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::VALUES.into_iter().any(|(_, b)| b == byte).then_some(
            /* SAFETY: byte is a valid escape code */
            unsafe { core::mem::transmute(byte) },
        )
    }
}
