#![cfg_attr(target_arch = "avr", no_std)]
#![feature(concat_bytes)]

mod transport;
pub use transport::Encoder;

mod device;
pub use device::Device;

mod escape;
use escape::EscapeCode;

mod bititer;
pub use bititer::BitIter;

mod bitvec;
pub use bitvec::BitVec;

mod connection;
pub use connection::Connection;
use ufmt::uwrite;

const CHECKSUM_LEN: usize = 0;
pub const FRAME_DATA_LEN: usize = 64;
pub const FRAME_LEN: usize = /* Escape code */
    1 + /* Len */ 1 + FRAME_DATA_LEN + CHECKSUM_LEN + /* Noop */  1;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Frame {
    pub len: u8,
    pub data: [u8; FRAME_DATA_LEN],
    checksum: [u8; CHECKSUM_LEN],
}

impl Frame {
    pub fn empty_invalid() -> Self {
        Self {
            len: 0,
            data: [0; FRAME_DATA_LEN],
            checksum: [0; CHECKSUM_LEN],
        }
    }

    pub fn new(len: u8, data: [u8; FRAME_DATA_LEN]) -> Self {
        Self {
            len,
            data,
            checksum: Self::checksum(&data),
        }
    }

    /// ## Structure of frame
    ///
    /// - SOF
    /// - index
    /// - data
    /// - checksums
    ///
    pub fn encode(&self) -> [u8; FRAME_LEN] {
        let mut bytes = [0; FRAME_LEN];

        bytes[0] = self.len;
        bytes[1] = EscapeCode::StartOfFrame as u8;
        bytes[2..2 + FRAME_DATA_LEN].copy_from_slice(&self.data);
        // bytes[2 + FRAME_DATA_LEN..].copy_from_slice(&self.checksum);
        bytes[FRAME_LEN - 1] = EscapeCode::Noop as u8;

        bytes
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut frame = Self {
            len: bytes[1],
            data: [0; FRAME_DATA_LEN],
            checksum: [0; CHECKSUM_LEN],
        };

        frame.data.copy_from_slice(&bytes[2..2 + FRAME_DATA_LEN]);
        // frame.checksum.copy_from_slice(&bytes[2 + FRAME_DATA_LEN..]);

        frame
    }

    // Check whether the checksum matches up with the data
    pub fn is_valid(&self) -> bool {
        todo!()
    }

    fn checksum(_data: &[u8; FRAME_DATA_LEN]) -> [u8; CHECKSUM_LEN] {
        [0; 0]
    }
}

impl ufmt::uDebug for Frame {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        uwrite!(f, "Frame[ ")?;
        for byte in self.data {
            uwrite!(f, "{:x} ", byte)?;
        }
        uwrite!(f, "]")
    }
}
