#![cfg_attr(target_arch = "avr", no_std)]

mod transport;
pub use transport::TransportEncode;

mod device;
pub use device::Device;

mod escape;
use escape::EscapeCode;

mod bititer;
pub use bititer::BitIter;

mod bitvec;
pub use bitvec::BitVec;

mod indexmap;
pub use indexmap::IndexMap;

mod connection;
pub use connection::{Connection, MirrorConnection};

const CHECKSUM_LEN: usize = 0;
const FRAME_DATA_LEN: usize = 64;
const FRAME_LEN: usize = /* Escape code */ 8 + /* Index */ 8 + FRAME_DATA_LEN + CHECKSUM_LEN;

#[derive(PartialEq, Eq)]
pub struct Frame {
    index: u8,
    data: [u8; FRAME_DATA_LEN],
    checksum: [u8; CHECKSUM_LEN],
}

/// ## Structure of frame
///
/// - SOF
/// - index
/// - data
/// - checksums
///
fn encode_frame(frame: Frame) -> [u8; FRAME_LEN] {
    let mut bytes = [0; FRAME_LEN];

    bytes[0] = EscapeCode::StartOfFrame as u8;
    bytes[1] = frame.index;
    bytes[2..2 + FRAME_DATA_LEN].copy_from_slice(&frame.data);
    bytes[2 + FRAME_DATA_LEN..].copy_from_slice(&frame.checksum);

    bytes
}

fn decode_frame(bytes: &[u8; FRAME_LEN]) -> Frame {
    let mut frame = Frame {
        index: bytes[1],
        data: [0; FRAME_DATA_LEN],
        checksum: [0; CHECKSUM_LEN],
    };

    frame.data.copy_from_slice(&bytes[2..2 + FRAME_DATA_LEN]);
    frame.checksum.copy_from_slice(&bytes[2 + FRAME_DATA_LEN..]);

    frame
}
