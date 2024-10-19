use std::io::{Bytes, Error, Read};

#[repr(u8)]
pub enum EscapeCode {
    /// SOF
    StartOfFrame = 0x12,
    /// EOF
    EndOfFrame = 0x23,
    /// CFD
    CorrectFrameData = 0x34,
    /// IFD
    IncorrectFrameData = 0x45,
    // NF
    NegateFollowing = 0x56,
    // FS
    FinishedSending = 0x67,
}

impl EscapeCode {
    const VALUES: [u8; 6] = [
        Self::StartOfFrame as u8,
        Self::EndOfFrame as u8,
        Self::CorrectFrameData as u8,
        Self::IncorrectFrameData as u8,
        Self::NegateFollowing as u8,
        Self::FinishedSending as u8,
    ];

    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::VALUES.contains(&byte).then_some(
            /* SAFETY: byte is a valid escape code */
            unsafe { std::mem::transmute(byte) },
        )
    }
}

pub struct EscapedBytes<R: Read> {
    bytes: Bytes<R>,
    /// Second half of an escaped value
    escape: Option<u8>,
    done: bool,
}

impl<R: Read> EscapedBytes<R> {
    pub fn new(bytes: Bytes<R>) -> Self {
        Self {
            bytes,
            escape: None,
            done: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl<R: Read> Iterator for EscapedBytes<R> {
    type Item = Result<u8, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.escape.take() {
            return Some(Ok(byte));
        }

        let result = self.bytes.next().inspect(|maybe_byte| {
            if let Ok(byte) = maybe_byte {
                if EscapeCode::VALUES.contains(&byte) {
                    let swapped_nibbles = (byte << 4) | (byte >> 4);
                    self.escape = Some(swapped_nibbles);
                }
            }
        });
        self.done = result.is_some();
        result
    }
}
