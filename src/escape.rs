use std::io::{Bytes, Error, Read};

#[repr(u8)]
pub enum EscapeCode {
    /// Start of frame
    SOF = 0x12,
    /// End of frame
    EOF = 0x23,
    /// Correct frame data
    CFD = 0x34,
    /// Incorrect frame data
    IFD = 0x45,
    // Negate previous nibble
    NPN = 0x56,
    // Done sending
    DS = 0x67,
}

impl EscapeCode {
    const VALUES: [u8; 6] = [
        Self::SOF as u8,
        Self::EOF as u8,
        Self::CFD as u8,
        Self::IFD as u8,
        Self::NPN as u8,
        Self::DS as u8,
    ];
}

pub struct EscapedBytes<R: Read> {
    bytes: Bytes<R>,
    /// Second half of an escaped value
    escape: Option<u8>,
}

impl<R: Read> EscapedBytes<R> {
    pub fn new(bytes: Bytes<R>) -> Self {
        Self {
            bytes,
            escape: None,
        }
    }
}

impl<R: Read> Iterator for EscapedBytes<R> {
    type Item = Result<u8, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.escape.take() {
            return Some(Ok(byte));
        }

        self.bytes.next().inspect(|maybe_byte| {
            if let Ok(byte) = maybe_byte {
                if EscapeCode::VALUES.contains(&byte) {
                    let swapped_nibbles = (byte << 4) | (byte >> 4);
                    self.escape = Some(swapped_nibbles);
                }
            }
        })
    }
}
