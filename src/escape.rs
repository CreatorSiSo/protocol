use std::io;

#[derive(Debug, PartialEq, Eq)]
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
    // BU1
    Buffer1 = 0x56,
    // BU2
    Buffer2 = 0x65,
    // FS
    FinishedSending = 0x67,
}

impl EscapeCode {
    const VALUES: [u8; 7] = [
        Self::StartOfFrame as u8,
        Self::EndOfFrame as u8,
        Self::CorrectFrameData as u8,
        Self::IncorrectFrameData as u8,
        Self::Buffer1 as u8,
        Self::Buffer2 as u8,
        Self::FinishedSending as u8,
    ];

    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::VALUES.contains(&byte).then_some(
            /* SAFETY: byte is a valid escape code */
            unsafe { std::mem::transmute(byte) },
        )
    }
}

pub struct Escaped<I: Iterator<Item = io::Result<u8>>> {
    bytes: I,
    /// Second half of an escaped value
    escape: Option<u8>,
    done: bool,
}

impl<I: Iterator<Item = io::Result<u8>>> Escaped<I> {
    pub fn new(bytes: I) -> Self {
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

impl<I: Iterator<Item = io::Result<u8>>> Iterator for Escaped<I> {
    type Item = Result<u8, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.escape.take() {
            return Some(Ok(byte));
        }

        let result = self.bytes.next().inspect(|maybe_byte| {
            if let Ok(byte) = maybe_byte {
                // Repeat value of escape code to escape it
                if EscapeCode::VALUES.contains(&byte) {
                    self.escape = Some(*byte);
                }
            }
        });
        self.done = result.is_some();
        result
    }
}
