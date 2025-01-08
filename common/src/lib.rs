#![cfg_attr(target_arch = "avr", no_std)]

mod transport;
pub use transport::TransportEncode;

mod device;
pub use device::Device;

mod escape;
use escape::EscapeCode;

mod bitvec;

const ESCAPE_CODE_LEN: usize = 1;
const CHECKSUM_LEN: usize = 0;
const FRAME_DATA_LEN: usize = 64;
const FRAME_LEN: usize = ESCAPE_CODE_LEN + FRAME_DATA_LEN + CHECKSUM_LEN + ESCAPE_CODE_LEN;
pub type Frame = [u8; FRAME_LEN];

/// # Steps
///
/// 1. calculate checksums
/// 2. add start of frame
/// 3. escape and add values
/// 4. add checksums
/// 5. add end of frame
///
/// ## Structure of frame
///
/// - SOF
/// - data
/// - checksums
/// - EOF
///
/// ## Calculating checksums
///
/// TODO
///
/// ## Encoding values equal to escape codes
///
/// | Function               | Escape code | Escaped value  |
/// | ---------------------- | ----------- | -------------- |
/// | start of frame         | (SOF) 0x12  | 0x12 0x12      |
/// | end of frame           | (EOF) 0x23  | 0x23 0x23      |
/// | correct frame data     | (CDF) 0x34  | 0x34 0x34      |
/// | incorrect frame data   | (IDF) 0x45  | 0x45 0x45      |
/// | buffer                 | (BU)  0x56  | 0x56 0x56      |
/// | finished sending       | (FS)  0x67  | 0x67 0x67      |
///
/// 0x56 0x65 0x9a 0x56
/// 0x56      0x9a 0x56
/// 0x56      0x65
///
fn encode_frame(data: &mut impl Iterator<Item = u8>) -> Frame {
    let mut frame = [0; FRAME_LEN];
    frame[0] = EscapeCode::StartOfFrame as u8;

    for cell in &mut frame[1..(1 + FRAME_DATA_LEN)] {
        *cell = match data.next() {
            Some(byte) => byte,
            // TODO Send finished escape code
            None => break,
        }
    }

    // TODO Encode chucksums

    frame[FRAME_LEN - 1] = EscapeCode::EndOfFrame as u8;

    frame
}

/// 1. calculate checksums for received data
/// 2. compare checksums
///
fn decode_frame(frame: &[u8; FRAME_DATA_LEN + CHECKSUM_LEN]) -> &[u8] {
    frame
}

#[derive(PartialEq, Eq)]
pub enum Command {
    Received([u8; FRAME_DATA_LEN + CHECKSUM_LEN]),
    SendNextFrame,
    ResendLastFrame,
    /// From now on the other side will only send escape codes
    StopReceivingData,
    None,
}

#[cfg(not(target_arch = "avr"))]
fn debug_bytes_hex(bytes: &[u8]) -> String {
    let mut result = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .zip(core::iter::repeat(", "))
        .fold(String::from("["), |accum, (l, r)| accum + &l + r);
    result.pop();
    result.pop();
    result + "]"
}

#[cfg(not(target_arch = "avr"))]
fn debug_bytes_binary(bytes: &[u8]) -> String {
    let mut result = bytes
        .iter()
        .map(|byte| format!("{byte:08b}"))
        .zip(core::iter::repeat(", "))
        .fold(String::from("["), |accum, (l, r)| accum + &l + r);
    result.pop();
    result.pop();
    result + "]"
}

#[cfg(not(target_arch = "avr"))]
impl core::fmt::Debug for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Received(arg0) => f
                .debug_tuple("Received")
                .field(&debug_bytes_hex(arg0))
                .finish(),
            Self::SendNextFrame => f.write_str("SendNextFrame"),
            Self::ResendLastFrame => f.write_str("ResendLastFrame"),
            Self::StopReceivingData => f.write_str("StopReceivingData"),
            Self::None => f.write_str("None"),
        }
    }
}
