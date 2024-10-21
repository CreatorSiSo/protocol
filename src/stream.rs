use crate::escape::{swap_nibbles, EscapeCode};
use crate::{Frame, CHECKSUM_LEN, FRAME_DATA_LEN};

pub struct InputStream {
    state: InputState,
    // the last 4 nibbles that have been received
    window: u16,
    // how many nibbles have been pushed into the window
    window_length: u8,
    data: [u8; FRAME_DATA_LEN + CHECKSUM_LEN],
    // index of nibble in the frame to write to next
    data_index: usize,
}

impl InputStream {
    pub fn new() -> Self {
        Self {
            state: InputState::WaitingForFrame,
            window: 0x0000,
            window_length: 0,
            data: [0; FRAME_DATA_LEN + CHECKSUM_LEN],
            data_index: 0,
        }
    }

    pub fn push(&mut self, nibble: u8) -> Command {
        match self.state {
            InputState::WaitingForFrame => self.waiting_for_frame(nibble),
            InputState::ReadingFrame => self.reading_frame(nibble),
        }
    }

    fn waiting_for_frame(&mut self, nibble: u8) -> Command {
        let should_read_window = self.window_push(nibble);
        if !should_read_window {
            return Command::None;
        }

        match dbg!(self.window_decode_value()) {
            DecodedValue::EscapeCode(escape_code) => match escape_code {
                EscapeCode::StartOfFrame => {
                    self.state = InputState::ReadingFrame;
                    eprintln!("Changed to {:?}", self.state);
                }
                EscapeCode::CorrectFrameData => return Command::SendNextFrame,
                EscapeCode::IncorrectFrameData => return Command::ResendLastFrame,
                EscapeCode::FinishedSending => return Command::StopReceivingData,
                EscapeCode::Buffer => unreachable!(),
                EscapeCode::EndOfFrame => eprintln!("Unexpected value"),
            },
            _ => (),
        }

        Command::None
    }

    fn reading_frame(&mut self, nibble: u8) -> Command {
        let changed = self.window_push(nibble);
        if !changed {
            return Command::None;
        }

        match self.window_decode_value() {
            DecodedValue::Nibble(value) => {
                eprintln!("_{:01x}", value);
                self.data[self.data_index / 2] |= value << ((1 + self.data_index) % 2) * 4;
                self.data_index += 1;
                Command::None
            }
            DecodedValue::Byte(value) => {
                eprintln!("{:02x}", value);
                self.data[self.data_index / 2] = value;
                self.data_index += 2;
                Command::None
            }
            DecodedValue::EscapeCode(escape_code) => {
                eprintln!("{:?}", escape_code);

                if !matches!(escape_code, EscapeCode::StartOfFrame) {
                    self.state = InputState::ReadingFrame;
                    eprintln!("State is now {:?}", self.state);
                }

                match escape_code {
                    EscapeCode::StartOfFrame if self.data_index != 0 => Command::ResendLastFrame,
                    EscapeCode::EndOfFrame => {
                        if self.data_index / 2 == self.data.len() {
                            self.data_index = 0;
                            Command::Received(self.data)
                        } else {
                            self.data_index = 0;
                            Command::ResendLastFrame
                        }
                    }
                    EscapeCode::CorrectFrameData => Command::SendNextFrame,
                    EscapeCode::IncorrectFrameData => Command::ResendLastFrame,
                    EscapeCode::FinishedSending => Command::StopReceivingData,

                    EscapeCode::StartOfFrame => Command::None,
                    EscapeCode::Buffer => unreachable!(),
                }
            }
        }
    }

    fn window_decode_value(&mut self) -> DecodedValue {
        let higher_byte = (self.window >> u8::BITS) as u8;
        let lower_byte = self.window as u8;

        // detect escape codes and shrink the window,
        // so that the data is not decoded again in the next iteration
        match EscapeCode::from_byte(higher_byte) {
            Some(_) if higher_byte == swap_nibbles(lower_byte) => {
                self.window_length = 0;
                let byte = self.window >> u8::BITS;
                DecodedValue::Byte(byte as u8)
            }
            Some(escape_code) => {
                self.window_length = 2;
                DecodedValue::EscapeCode(escape_code)
            }
            None => {
                self.window_length = 3;
                let nibble = self.window >> (u8::BITS + u8::BITS / 2);
                DecodedValue::Nibble(nibble as u8)
            }
        }
    }

    /// Pushes the nibble into the window and
    /// returns whether the window should be looked at or not
    fn window_push(&mut self, nibble: u8) -> bool {
        // ensures that the unused nibble is 0
        let nibble = nibble & 0x0f;
        // truncates the u16, so that only the least significant nibble is left
        let previous_nibble = (self.window as u8) & 0x0f;
        // whether value has changed
        if previous_nibble == nibble {
            return false;
        }

        eprintln!("{nibble:04b}");

        // push received nibble
        self.window <<= 4;
        self.window |= nibble as u16;
        self.window_length += 1;

        // whether enough data has been pushed into the window
        self.window_length == 4
    }
}

#[derive(Debug)]
enum DecodedValue {
    Nibble(u8),
    Byte(u8),
    EscapeCode(EscapeCode),
}

#[derive(Debug)]
enum InputState {
    WaitingForFrame,
    ReadingFrame,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Received([u8; FRAME_DATA_LEN + CHECKSUM_LEN]),
    SendNextFrame,
    ResendLastFrame,
    /// From now on the other side will only send escape codes
    StopReceivingData,
    None,
}

#[test]
fn read_alternating() {
    let bytes = [0xf0; 64];

    let (commands, _) = use_input_stream(bytes.into_iter());
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Received(..)))
            .collect::<Vec<_>>(),
        vec![&Command::Received([0xf0; 64])],
    );
}

#[test]
fn read_zeros() {
    let bytes = [0x00; 64];

    let (commands, _) = use_input_stream(bytes.into_iter());
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Received(..)))
            .collect::<Vec<_>>(),
        vec![&Command::Received([0x00; 64])],
    );
}

#[test]
fn read_random() {
    let bytes = [
        0xa0, 0x8e, 0x4f, 0x24, 0x68, 0x53, 0x13, 0xcb, 0x17, 0xeb, 0xa1, 0xf2, 0x7e, 0xb3, 0xab,
        0x07, 0x00, 0x4c, 0xac, 0x54, 0x34, 0x5b, 0x72, 0x96, 0x09, 0xc0, 0xda, 0xbc, 0x17, 0xbc,
        0xef, 0xa9, 0x7f, 0x65, 0x39, 0x58, 0x21, 0x72, 0xdd, 0x0b, 0xba, 0x9a, 0x75, 0xcd, 0x5f,
        0xa2, 0x44, 0x43, 0x1b, 0xd2, 0x0d, 0x5b, 0x7c, 0x65, 0xbb, 0xc9, 0x4f, 0x78, 0xfe, 0x08,
        0x6e, 0x23, 0xce, 0x40,
    ];

    let (commands, _) = use_input_stream(bytes.into_iter());
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Received(..)))
            .collect::<Vec<_>>(),
        vec![&Command::Received(bytes)],
    );
}

#[cfg(test)]
fn use_input_stream(data: impl Iterator<Item = u8>) -> (Vec<Command>, InputStream) {
    use crate::{encode_frame, Escaped};
    use std::iter;

    let mut iter = Escaped::new(data.map(|byte| Ok(byte)));

    let mut input_stream = InputStream::new();
    let mut commands = Vec::new();

    while !iter.is_done() {
        let frame = encode_frame(&mut iter);
        eprintln!(
            "{}",
            frame
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .zip(iter::repeat(", "))
                .fold(String::new(), |accum, (l, r)| accum + &l + r)
        );

        for byte in [&[0xf0; 5], frame.as_slice(), &[0xf0; 5]].concat() {
            commands.push(input_stream.push(byte >> 4));
            commands.push(input_stream.push(byte));
        }
    }

    return (commands, input_stream);
}

pub struct OutputStream {
    /// Data to send
    frame: Frame,
    /// Index of the nibble to send
    index: usize,
}

impl OutputStream {
    pub fn new(frame: Frame) -> Self {
        eprintln!("{:?}", frame);
        Self { frame, index: 0 }
    }

    /// returns the next nibble to send
    pub fn pull(&mut self) -> Option<u8> {
        if self.index >= (self.frame.len() * 2) {
            return None;
        }

        let mut byte = self.frame[self.index / 2];
        if self.index % 2 == 0 {
            byte >>= 4;
        } else {
            byte &= 0x0f;
        };
        self.index += 1;

        Some(byte)
    }

    /// Resets the internal state, but keeps the frame data.
    pub fn reset(&mut self) {
        self.index = 0;
    }
}
