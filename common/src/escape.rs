use crate::{bititer::Byte, BitVec};

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EscapeCode {
    // SNC
    Sync = 0xf0,
    /// SOF
    StartOfFrame = 0x33,
    /// ACK
    Ack = 0x44,
    /// NCK
    Nack = 0x55,
    // FS
    FinishedSending = 0x66,
}

impl EscapeCode {
    const VALUES: [(EscapeCode, u8); 5] = [
        (Self::Sync, Self::Sync as u8),
        (Self::StartOfFrame, Self::StartOfFrame as u8),
        (Self::Ack, Self::Ack as u8),
        (Self::Nack, Self::Nack as u8),
        (Self::FinishedSending, Self::FinishedSending as u8),
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
