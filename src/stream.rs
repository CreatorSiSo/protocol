use crate::escape::EscapeCode;
use crate::{Frame, CHECKSUM_LEN, FRAME_DATA_LEN, FRAME_LEN};
use std::fmt::Debug;

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

        match self.window_decode_value() {
            DecodedValue::EscapeCode(escape_code) => match escape_code {
                EscapeCode::StartOfFrame => {
                    self.state = InputState::ReadingFrame;
                    eprintln!("State is now {:?}", self.state);
                }
                EscapeCode::CorrectFrameData => return Command::SendNextFrame,
                EscapeCode::IncorrectFrameData => return Command::ResendLastFrame,
                EscapeCode::FinishedSending => return Command::StopReceivingData,
                EscapeCode::Buffer1 | EscapeCode::Buffer2 => eprintln!("Unexpected value"),
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

        let value = self.window_decode_value();
        eprintln!("decoded: {:?}, index: {}", value, self.data_index);
        match value {
            DecodedValue::Nibble(value) => {
                // eprintln!("_{:01x}", value);
                self.data[self.data_index / 2] |= value << ((1 + self.data_index) % 2) * 4;
                self.data_index += 1;
                Command::None
            }
            DecodedValue::Byte(value) => {
                // eprintln!("{:02x}", value);
                self.data[self.data_index / 2] = value;
                self.data_index += 2;
                Command::None
            }
            DecodedValue::EscapeCode(escape_code) => {
                if !matches!(escape_code, EscapeCode::StartOfFrame) {
                    self.state = InputState::ReadingFrame;
                    eprintln!("State is now {:?}", self.state);
                }

                match dbg!(&escape_code) {
                    EscapeCode::StartOfFrame if self.data_index != 0 => Command::ResendLastFrame,
                    EscapeCode::EndOfFrame => {
                        if dbg!(dbg!(self.data_index / 2) == self.data.len()) {
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
                    EscapeCode::StartOfFrame | EscapeCode::Buffer1 | EscapeCode::Buffer2 => {
                        Command::None
                    }
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
            Some(_) if higher_byte == lower_byte => {
                self.window_length = 0;
                let byte = self.window >> u8::BITS;
                DecodedValue::Byte(byte as u8)
            }
            Some(escape_code) => {
                eprintln!("window = {:04x}", self.window);
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

        // push received nibble
        self.window <<= 4;
        self.window |= nibble as u16;
        self.window_length += 1;

        // whether enough data has been pushed into the window
        self.window_length == 4
    }
}

enum DecodedValue {
    Nibble(u8),
    Byte(u8),
    EscapeCode(EscapeCode),
}

impl Debug for DecodedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nibble(arg0) => f
                .debug_tuple("Nibble")
                .field(&format!("{:01x}", arg0))
                .finish(),
            Self::Byte(arg0) => f
                .debug_tuple("Byte")
                .field(&format!("{:02x}", arg0))
                .finish(),
            Self::EscapeCode(arg0) => f.debug_tuple("EscapeCode").field(arg0).finish(),
        }
    }
}

#[derive(Debug)]
enum InputState {
    WaitingForFrame,
    ReadingFrame,
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

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received(arg0) => f
                .debug_tuple("Received")
                .field(&bytes_to_debug_string(arg0))
                .finish(),
            Self::SendNextFrame => write!(f, "SendNextFrame"),
            Self::ResendLastFrame => write!(f, "ResendLastFrame"),
            Self::StopReceivingData => write!(f, "StopReceivingData"),
            Self::None => write!(f, "None"),
        }
    }
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
        vec![&Command::Received([
            0xa0, 0x8e, 0x4f, 0x24, 0x68, 0x53, 0x13, 0xcb, 0x17, 0xeb, 0xa1, 0xf2, 0x7e, 0xb3,
            0xab, 0x07, 0x00, 0x4c, 0xac, 0x54, 0x34, 0x34, 0x5b, 0x72, 0x96, 0x09, 0xc0, 0xda,
            0xbc, 0x17, 0xbc, 0xef, 0xa9, 0x7f, 0x65, 0x39, 0x58, 0x21, 0x72, 0xdd, 0x0b, 0xba,
            0x9a, 0x75, 0xcd, 0x5f, 0xa2, 0x44, 0x43, 0x1b, 0xd2, 0x0d, 0x5b, 0x7c, 0x65, 0xbb,
            0xc9, 0x4f, 0x78, 0xfe, 0x08, 0x6e, 0x23, 0x23,
        ])],
    );
}

#[cfg(test)]
fn use_input_stream(data: impl Iterator<Item = u8>) -> (Vec<Command>, InputStream) {
    use crate::{encode_frame, Escaped};
    let mut iter = Escaped::new(data.map(|byte| Ok(byte)));

    let mut output_stream = OutputStream::new();
    let mut input_stream = InputStream::new();
    let mut commands = Vec::new();

    while !iter.is_done() {
        let frame = encode_frame(&mut iter);
        eprintln!("{}", bytes_to_debug_string(&frame));

        // TODO Use output stream
        for byte in [&[0xf0; 5], frame.as_slice(), &[0xf0; 5]].concat() {
            let higher_nibble = byte >> 4;
            let lowher_nibble = byte & 0x0f;
            commands.push(input_stream.push(higher_nibble));
            if higher_nibble == lowher_nibble {
                commands.push(input_stream.push(EscapeCode::Buffer1 as u8 >> 4));
                commands.push(input_stream.push(EscapeCode::Buffer1 as u8));
            }
            commands.push(input_stream.push(lowher_nibble));
            commands.push(input_stream.push(EscapeCode::Buffer1 as u8 >> 4));
            commands.push(input_stream.push(EscapeCode::Buffer1 as u8));
        }
    }

    return (commands, input_stream);
}

fn bytes_to_debug_string(bytes: &[u8]) -> String {
    let mut result = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .zip(std::iter::repeat(", "))
        .fold(String::from("["), |accum, (l, r)| accum + &l + r);
    result.pop();
    result.pop();
    result + "]"
}

enum OutputState {
    WaitingForFrame,
    WritingFrame,
}

pub struct OutputStream {
    state: OutputState,
    /// Data to send
    frame: Frame,
    /// Index of the nibble to send
    index: usize,
    window: Window<4>,
}

impl OutputStream {
    pub fn new() -> Self {
        Self {
            state: OutputState::WaitingForFrame,
            frame: [0; FRAME_LEN],
            index: 0,
            window: Window::new(),
        }
    }

    pub fn send_frame(&mut self, frame: Frame) {
        self.state = OutputState::WritingFrame;
        self.frame = frame;
        self.index = 0;
    }

    /// Resets the internal state, but keeps the frame data.
    pub fn resend_frame(&mut self) {
        self.state = OutputState::WaitingForFrame;
        self.index = 0;
    }

    /// returns the next nibble to send
    pub fn next(&mut self) -> u8 {
        match self.state {
            OutputState::WaitingForFrame => self.waiting_for_frame(),
            OutputState::WritingFrame => {
                if let Some(nibble) = self.writing_frame() {
                    nibble
                } else {
                    self.state = OutputState::WaitingForFrame;
                    self.waiting_for_frame()
                }
            }
        }
    }

    fn waiting_for_frame(&mut self) -> u8 {
        let nibble = if self.index % 2 == 0 { 0x0f } else { 0x00 };
        self.index += 1;
        nibble
    }

    fn writing_frame(&mut self) -> Option<u8> {
        if let Some(byte) = self.frame.get(self.index / 2) {
            let nibble = if self.index % 2 == 0 {
                byte >> 4
            } else {
                byte & 0x0f
            };
            self.window.push_back(nibble);
        }

        if self.window.len > 2 {
            self.window.pop_front()
        } else if self.window.len == 2 {
            let higher = self.window.get(1).expect("upper nibble");
            let lower = self.window.get(0).expect("lower nibble");
            let escape_code = if higher == EscapeCode::Buffer1 as u8 >> 4 {
                EscapeCode::Buffer2 as u8
            } else {
                EscapeCode::Buffer1 as u8 >> 4
            };

            if higher == lower {
                self.window.pop_back();
                self.window.push_back(escape_code >> 4);
                self.window.push_back(escape_code & 0x0f);
                self.window.push_back(lower);
            }
            self.window.pop_front()
        } else {
            None
        }
    }
}

/// # Window
///
/// A vector-like data structure with a maximum length, that stores nibbles.
///
/// ## Layout
///
/// byte index:   <  0,   1,   2,   3    >
/// nibble index: <  7 6, 5 4, 3 2, 1 0  >
/// data:         [  x x, x x, x x, x x  ]
///
/// push(1): [   xx,  xx,  xx,   x1  ]
/// push(2): [   xx,  xx,  xx,   12  ]
/// push(3): [   xx,  xx,  x1,   23  ]
/// push(4): [   xx,  xx,  12,   34  ]
/// push(5): [   xx,  x1,  23,   45  ]
///
/// N is the underlying maximum size in bytes
struct Window<const N: usize> {
    // underlying bytes
    data: [u8; N],
    // length in nibbles
    len: usize,
}

impl<const N: usize> Window<N> {
    fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    fn push_back(&mut self, nibble: u8) -> Option<u8> {
        let prev = self.data;

        // Shift every nibble 4 bits to the left
        for (index, byte) in self.data.iter_mut().enumerate() {
            *byte <<= 4;
            *byte |= prev
                .get(index + 1)
                .map(|byte_to_right| byte_to_right >> 4)
                .unwrap_or(nibble);
        }

        let filled = self.len / 2 == self.data.len();
        if !filled {
            self.len += 1;
        }

        filled.then_some(prev[0] >> 4)
    }

    fn pop_front(&mut self) -> Option<u8> {
        let result = self.get(self.len - 1);
        if self.len > 0 {
            self.len -= 1;
        }
        result
    }

    fn pop_back(&mut self) -> Option<u8> {
        let prev = self.data;

        // Shift every nibble 4 bits to the right
        for (index, byte) in self.data.iter_mut().enumerate() {
            *byte >>= 4;
            *byte |= index
                .checked_sub(1)
                .map(|index_to_left| prev[index_to_left] << 4)
                .unwrap_or(0x00);
        }

        let not_empty = self.len > 0;
        if not_empty {
            self.len -= 1;
        }

        not_empty.then_some(prev[prev.len() - 1] & 0x0f)
    }

    fn get(&self, index: usize) -> Option<u8> {
        if index >= self.len {
            return None;
        }

        let byte_index = self.data.len() - 1 - index / 2;
        let byte = self.data[byte_index];
        let nibble = if index % 2 == 0 {
            byte & 0x0f
        } else {
            byte >> 4
        };
        Some(nibble)
    }
}

#[test]
fn window() {
    let mut window = Window::<2>::new();
    assert_eq!(
        [None, None, None, None, Some(0x01)],
        [
            window.push_back(0x01),
            window.push_back(0x02),
            window.push_back(0x03),
            window.push_back(0x04),
            window.push_back(0x05)
        ]
    );
    assert_eq!([0x23, 0x45], window.data);
    assert_eq!(
        [Some(0x05), Some(0x04), Some(0x03), Some(0x02), None],
        [
            window.get(0),
            window.get(1),
            window.get(2),
            window.get(3),
            window.get(4)
        ]
    );
    assert_eq!(
        [Some(0x05), Some(0x04), Some(0x03), Some(0x02), None],
        [
            window.pop_back(),
            window.pop_back(),
            window.pop_back(),
            window.pop_back(),
            window.pop_back()
        ]
    );
}
