#![cfg_attr(target_arch = "avr", no_std)]

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
use ufmt::{uwrite, uwriteln};

const CHECKSUM_LEN: usize = 4;
pub const FRAME_DATA_LEN: usize = 64;
pub const FRAME_PAYLOAD_LEN: usize = /* Len */ 1 + FRAME_DATA_LEN;
pub const FRAME_LEN: usize = /* Escape code */
    1 + FRAME_PAYLOAD_LEN + CHECKSUM_LEN + /* Noop */  1;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Frame {
    payload: [u8; FRAME_PAYLOAD_LEN],
    checksum: [u8; CHECKSUM_LEN],
}

impl Frame {
    pub fn empty_invalid() -> Self {
        Self {
            payload: [0; FRAME_PAYLOAD_LEN],
            checksum: [0; CHECKSUM_LEN],
        }
    }

    pub fn new(len: u8, data: [u8; FRAME_DATA_LEN]) -> Self {
        let mut combined = [0; FRAME_PAYLOAD_LEN];
        combined[0] = len;
        combined[1..].copy_from_slice(&data);

        Self {
            payload: combined,
            checksum: Self::checksum(&combined),
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

        bytes[0] = EscapeCode::StartOfFrame as u8;
        bytes[1..1 + FRAME_PAYLOAD_LEN].copy_from_slice(&self.payload);
        bytes[1 + FRAME_PAYLOAD_LEN..FRAME_LEN - 1].copy_from_slice(&self.checksum);
        bytes[FRAME_LEN - 1] = EscapeCode::Noop as u8;

        bytes
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut frame = Self {
            payload: [0; 1 + FRAME_DATA_LEN],
            checksum: [0; CHECKSUM_LEN],
        };

        frame
            .payload
            .copy_from_slice(&bytes[1..1 + FRAME_PAYLOAD_LEN]);
        frame
            .checksum
            .copy_from_slice(&bytes[1 + FRAME_PAYLOAD_LEN..FRAME_LEN - 1]);

        frame
    }

    // Check whether the checksum matches up with the data
    pub fn is_valid(&self) -> bool {
        self.checksum == Self::checksum(&self.payload)
    }

    // Compute the CRC32 checksum for the frame data
    fn checksum(combined: &[u8; 1 + FRAME_DATA_LEN]) -> [u8; CHECKSUM_LEN] {
        // Create a CRC32 instance using ISO_HDLC polynomial (this is standard CRC32)
        let crc = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
        let crc32 = crc.checksum(combined);
        crc32.to_le_bytes()
    }

    pub fn data(&self) -> &[u8] {
        &self.payload[1..1 + self.len() as usize]
    }

    pub fn len(&self) -> u8 {
        self.payload[0]
    }
}

impl core::fmt::Debug for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frame(len: {}, checksum: {})[ ",
            self.len(),
            u32::from_le_bytes(self.checksum)
        )?;
        for byte in self.data() {
            write!(f, "{:x} ", byte)?;
        }
        write!(f, "]\n")
    }
}

impl ufmt::uDebug for Frame {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        uwrite!(
            f,
            "Frame(len: {}, checksum: {})[ ",
            self.len(),
            u32::from_le_bytes(self.checksum)
        )?;
        for byte in self.data() {
            uwrite!(f, "{:x} ", *byte)?;
        }
        uwriteln!(f, "]\r")
    }
}

#[test]
fn frame_full_cycle() {
    let mut data = [0; FRAME_DATA_LEN];
    data[0] = 255;
    data[29] = 28;
    let len = 40;

    let frame = Frame::new(len, data);
    assert!(frame.is_valid());
    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded);
    assert_eq!(frame, decoded);
    assert!(decoded.is_valid());
}
