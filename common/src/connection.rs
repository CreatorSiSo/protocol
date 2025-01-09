use crate::{
    transport::TransportDecode, BitVec, Device, TransportEncode, CHECKSUM_LEN, FRAME_DATA_LEN,
};

#[cfg(target_arch = "avr")]
macro_rules! dbg {
    () => {};
    ($val:expr $(,)?) => {
        match $val {
            val => val,
        }
    };
    ($($val:expr),+ $(,)?) => {
        $val
    };
}

#[cfg(target_arch = "avr")]
fn log(_str: &'static str) {}

#[cfg(not(target_arch = "avr"))]
fn log(str: &'static str) {
    eprintln!("{}", str);
}

pub struct MirrorConnection<D: Device> {
    output_state: OutputState,
    encoder: TransportEncode,
    input_state: InputState,
    decoder: TransportDecode,
    device: D,
    failures: u16,
    successes: u8,
    received: BitVec<64>,
}

impl<D: Device> MirrorConnection<D> {
    pub fn new(device: D) -> Self {
        Self {
            output_state: OutputState::WaitingForFrame,
            encoder: TransportEncode::new(),
            input_state: InputState::WaitingForConnection,
            decoder: TransportDecode::new(),
            device,
            failures: 0,
            successes: 0,
            received: BitVec::new(),
        }
    }
}

impl<D: Device> Connection for MirrorConnection<D> {
    fn poll(&mut self) {
        if self.input_state == InputState::WaitingForConnection {
            self.encoder.establish_connection(&mut self.device);

            let successful = self.decoder.establish_connection(&mut self.device);
            if successful {
                self.successes += 1;
                self.failures = 0;
                if self.successes > 10 {
                    self.input_state_transition(InputState::WaitingForFrame);
                }
            } else {
                self.successes = 0;
                self.failures += 1;
                if self.failures > 3000 {
                    panic!("Could not establish connection");
                }
            }

            return;
        }

        if let Some(byte) = self.received.pop_front(8) {
            self.encoder.push(&byte);
        }

        self.encoder.poll(&mut self.device);
        self.decoder.poll(&mut self.device);

        if let Some(byte) = self.decoder.read() {
            self.received.push_back(&byte);
        }
    }

    fn input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }

    fn output_state_mut(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
}

pub trait Connection {
    fn poll(&mut self);
    fn input_state_mut(&mut self) -> &mut InputState;
    fn output_state_mut(&mut self) -> &mut OutputState;

    fn input_state_transition(&mut self, next: InputState) {
        use InputState::*;
        let state = self.input_state_mut();

        dbg!((&state, &next));

        match (&state, &next) {
            (WaitingForConnection, WaitingForFrame) => log("=== Input: Established connection ==="),
            (ReadingFrame, WaitingForFrame) => log("=== Input: Waiting for frame ==="),
            (WaitingForFrame, ReadingFrame) => log("=== Input: Reading frame ==="),
            (ReadingFrame, ReadingFrame)
            | (WaitingForConnection, WaitingForConnection)
            | (WaitingForConnection, ReadingFrame)
            | (WaitingForFrame, WaitingForConnection)
            | (WaitingForFrame, WaitingForFrame)
            | (ReadingFrame, WaitingForConnection) => unreachable!(),
        }

        *state = next;
    }

    fn output_state_transition(&mut self, next: OutputState) {
        use OutputState::*;
        let state = self.output_state_mut();

        match (&state, &next) {
            (WaitingForFrame, WritingFrame) => log("=== Output: Writing frame ==="),
            (WritingFrame, WaitingForFrame) => log("=== Output: Waiting for frame ==="),
            (WaitingForFrame, WaitingForFrame) | (WritingFrame, WritingFrame) => unreachable!(),
        }

        *state = next;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutputState {
    WaitingForFrame,
    WritingFrame,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputState {
    WaitingForConnection,
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

// pub struct InputStream {
//     state: InputState,
//     // the last 4 nibbles that have been received
//     window: Window<2>,
//     data: [u8; FRAME_DATA_LEN + CHECKSUM_LEN],
//     // index of nibble in the frame to write to next
//     data_index: usize,
//     clock: usize,
// }

// impl InputStream {
//     pub fn new() -> Self {
//         Self {
//             state: InputState::WaitingForConnection,
//             window: Window::new(),
//             data: [0; FRAME_DATA_LEN + CHECKSUM_LEN],
//             data_index: 0,
//             clock: 0,
//         }
//     }

//     pub fn push(&mut self, nibble: u8) -> Command {
//         // whether the probed value on the cable has changed
//         if self.window.get(0) == Some(nibble) {
//             return Command::None;
//         }

//         self.window.push_back(nibble);

//         // whether enough data has been pushed into the window
//         if self.window.len < 4 {
//             return Command::None;
//         }

//         match self.state {
//             InputState::WaitingForConnection => self.waiting_for_connection(),
//             InputState::WaitingForFrame => self.waiting_for_frame(),
//             InputState::ReadingFrame => self.reading_frame(),
//         }
//     }

//     fn waiting_for_connection(&mut self) -> Command {
//         self.clock += 1;
//         if self.clock > 10 {
//             self.state_transition(InputState::WaitingForFrame);
//             Command::SendNextFrame
//         } else {
//             Command::None
//         }
//     }

//     fn waiting_for_frame(&mut self) -> Command {
//         match self.window_decode_value() {
//             DecodedValue::EscapeCode(escape_code) => match escape_code {
//                 EscapeCode::StartOfFrame => {
//                     self.state_transition(InputState::ReadingFrame);
//                 }
//                 EscapeCode::CorrectFrameData => return Command::SendNextFrame,
//                 EscapeCode::IncorrectFrameData => return Command::ResendLastFrame,
//                 EscapeCode::FinishedSending => return Command::StopReceivingData,
//                 EscapeCode::Buffer1 | EscapeCode::Buffer2 => eprintln!("Unexpected value"),
//                 EscapeCode::EndOfFrame => eprintln!("Unexpected value"),
//             },
//             _ => (),
//         }

//         Command::None
//     }

//     fn reading_frame(&mut self) -> Command {
//         let value = self.window_decode_value();
//         eprintln!("decoded: {:?}, index: {}", value, self.data_index);
//         match value {
//             DecodedValue::Nibble(value) => {
//                 self.data[self.data_index / 2] |= value << ((1 + self.data_index) % 2) * 4;
//                 self.data_index += 1;
//                 Command::None
//             }
//             DecodedValue::Byte(value) => {
//                 self.data[self.data_index / 2] = value;
//                 self.data_index += 2;
//                 Command::None
//             }
//             DecodedValue::EscapeCode(escape_code) => {
//                 if !matches!(escape_code, EscapeCode::StartOfFrame) {
//                     self.state_transition(InputState::ReadingFrame);
//                 }

//                 match &escape_code {
//                     EscapeCode::StartOfFrame if self.data_index != 0 => Command::ResendLastFrame,
//                     EscapeCode::EndOfFrame => {
//                         if dbg!(dbg!(self.data_index / 2) == self.data.len()) {
//                             self.data_index = 0;
//                             Command::Received(self.data)
//                         } else {
//                             self.data_index = 0;
//                             Command::ResendLastFrame
//                         }
//                     }
//                     EscapeCode::CorrectFrameData => Command::SendNextFrame,
//                     EscapeCode::IncorrectFrameData => Command::ResendLastFrame,
//                     EscapeCode::FinishedSending => Command::StopReceivingData,
//                     EscapeCode::StartOfFrame | EscapeCode::Buffer1 | EscapeCode::Buffer2 => {
//                         Command::None
//                     }
//                 }
//             }
//         }
//     }

//     fn window_decode_value(&mut self) -> DecodedValue {
//         let higher_byte = self.window.data[0];
//         let lower_byte = self.window.data[1];

//         // detect escape codes and shrink the window,
//         // so that the data is not decoded again in the next iteration
//         match EscapeCode::from_byte(higher_byte) {
//             Some(_) if higher_byte == lower_byte => {
//                 self.window.shrink(0);
//                 DecodedValue::Byte(higher_byte)
//             }
//             Some(escape_code) => {
//                 self.window.shrink(2);
//                 DecodedValue::EscapeCode(escape_code)
//             }
//             None => DecodedValue::Nibble(self.window.pop_front().unwrap()),
//         }
//     }
// }

// enum DecodedValue {
//     Nibble(u8),
//     Byte(u8),
//     EscapeCode(EscapeCode),
// }

// impl Debug for DecodedValue {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Nibble(arg0) => f
//                 .debug_tuple("Nibble")
//                 .field(&format!("{:01x}", arg0))
//                 .finish(),
//             Self::Byte(arg0) => f
//                 .debug_tuple("Byte")
//                 .field(&format!("{:02x}", arg0))
//                 .finish(),
//             Self::EscapeCode(arg0) => f.debug_tuple("EscapeCode").field(arg0).finish(),
//         }
//     }
// }

// #[derive(Debug)]
// enum InputState {
//     WaitingForConnection,
//     WaitingForFrame,
//     ReadingFrame,
// }

// #[derive(PartialEq, Eq)]
// pub enum Command {
//     Received([u8; FRAME_DATA_LEN + CHECKSUM_LEN]),
//     SendNextFrame,
//     ResendLastFrame,
//     /// From now on the other side will only send escape codes
//     StopReceivingData,
//     None,
// }

// impl Debug for Command {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Received(arg0) => f
//                 .debug_tuple("Received")
//                 .field(&bytes_to_debug_string(arg0))
//                 .finish(),
//             Self::SendNextFrame => write!(f, "SendNextFrame"),
//             Self::ResendLastFrame => write!(f, "ResendLastFrame"),
//             Self::StopReceivingData => write!(f, "StopReceivingData"),
//             Self::None => write!(f, "None"),
//         }
//     }
// }

// #[test]
// fn read_alternating() {
//     let bytes = [0xf0; 64];

//     let (stdout, _) = use_input_stream(bytes.into_iter());
//     assert_eq!(stdout, &[0xf0; 64]);
// }

// #[test]
// fn read_zeros() {
//     let bytes = [0x00; 64];

//     let (stdout, stream) = use_input_stream(bytes.into_iter());
//     dbg!(&stdout);
//     assert_eq!(stdout, &[0x00; 64]);
// }

// #[test]
// fn read_random() {
//     let bytes = [
//         0xa0, 0x8e, 0x4f, 0x24, 0x68, 0x53, 0x13, 0xcb, 0x17, 0xeb, 0xa1, 0xf2, 0x7e, 0xb3, 0xab,
//         0x07, 0x00, 0x4c, 0xac, 0x54, 0x34, 0x5b, 0x72, 0x96, 0x09, 0xc0, 0xda, 0xbc, 0x17, 0xbc,
//         0xef, 0xa9, 0x7f, 0x65, 0x39, 0x58, 0x21, 0x72, 0xdd, 0x0b, 0xba, 0x9a, 0x75, 0xcd, 0x5f,
//         0xa2, 0x44, 0x43, 0x1b, 0xd2, 0x0d, 0x5b, 0x7c, 0x65, 0xbb, 0xc9, 0x4f, 0x78, 0xfe, 0x08,
//         0x6e, 0x23, 0xce, 0x40,
//     ];

//     let (stdout, _) = use_input_stream(bytes.into_iter());
//     assert_eq!(
//         stdout,
//         [
//             0xa0, 0x8e, 0x4f, 0x24, 0x68, 0x53, 0x13, 0xcb, 0x17, 0xeb, 0xa1, 0xf2, 0x7e, 0xb3,
//             0xab, 0x07, 0x00, 0x4c, 0xac, 0x54, 0x34, 0x34, 0x5b, 0x72, 0x96, 0x09, 0xc0, 0xda,
//             0xbc, 0x17, 0xbc, 0xef, 0xa9, 0x7f, 0x65, 0x39, 0x58, 0x21, 0x72, 0xdd, 0x0b, 0xba,
//             0x9a, 0x75, 0xcd, 0x5f, 0xa2, 0x44, 0x43, 0x1b, 0xd2, 0x0d, 0x5b, 0x7c, 0x65, 0xbb,
//             0xc9, 0x4f, 0x78, 0xfe, 0x08, 0x6e, 0x23, 0x23,
//         ],
//     );
// }

// #[cfg(test)]
// fn use_input_stream(data: impl Iterator<Item = u8>) -> (Vec<u8>, InputStream) {
//     use crate::{
//         device::{DebugDevice, Device},
//         Connection, Escaped,
//     };

//     let bytes = Escaped::new(data.map(|byte| Ok(byte)));
//     let mut connection = Connection::new(DebugDevice::new(), bytes);
//     let mut stdout = Vec::new();

//     while connection.poll(&mut stdout) {}

//     return (stdout, connection.i_stream);
// }

// fn bytes_to_debug_string(bytes: &[u8]) -> String {
//     let mut result = bytes
//         .iter()
//         .map(|byte| format!("{byte:02x}"))
//         .zip(std::iter::repeat(", "))
//         .fold(String::from("["), |accum, (l, r)| accum + &l + r);
//     result.pop();
//     result.pop();
//     result + "]"
// }

// #[derive(PartialEq, Eq)]
// enum OutputState {
//     WaitingForFrame,
//     WritingFrame,
// }

// pub struct OutputStream {
//     state: OutputState,
//     /// Data to send
//     frame: Frame,
//     /// Index of the nibble to send
//     index: usize,
//     window: Window<4>,
// }

// impl OutputStream {
//     pub fn new() -> Self {
//         Self {
//             state: OutputState::WaitingForFrame,
//             frame: [0; FRAME_LEN],
//             index: 0,
//             window: Window::new(),
//         }
//     }

//     pub fn send_frame(&mut self, frame: Frame) {
//         self.state_transition(OutputState::WritingFrame);
//         dbg!(frame);
//         self.frame = frame;
//         self.index = 0;
//     }

//     /// Resets the internal state, but keeps the frame data.
//     pub fn resend_frame(&mut self) {
//         self.state_transition(OutputState::WritingFrame);
//         self.index = 0;
//     }

//     /// returns the next nibble to send
//     pub fn next(&mut self) -> u8 {
//         match self.state {
//             OutputState::WaitingForFrame => self.waiting_for_frame(),
//             OutputState::WritingFrame => {
//                 if let Some(nibble) = self.next_nibble() {
//                     nibble
//                 } else {
//                     self.waiting_for_frame()
//                 }
//             }
//         }
//     }

//     fn waiting_for_frame(&mut self) -> u8 {
//         let nibble = if self.index % 2 == 0 { 0x0f } else { 0x00 };
//         self.index += 1;
//         nibble
//     }

//     fn next_nibble(&mut self) -> Option<u8> {
//         if let Some(byte) = self.frame.get(self.index / 2) {
//             let nibble = if self.index % 2 == 0 {
//                 byte >> 4
//             } else {
//                 byte & 0x0f
//             };
//             self.window.push_back(nibble);
//         }

//         if self.window.len > 2 {
//             self.window.pop_front()
//         } else if self.window.len == 2 {
//             let higher = self.window.get(1).expect("upper nibble");
//             let lower = self.window.get(0).expect("lower nibble");
//             let escape_code = if higher == EscapeCode::Buffer1 as u8 >> 4 {
//                 EscapeCode::Buffer2 as u8
//             } else {
//                 EscapeCode::Buffer1 as u8 >> 4
//             };

//             if higher == lower {
//                 self.window.pop_back();
//                 self.window.push_back(escape_code >> 4);
//                 self.window.push_back(escape_code & 0x0f);
//                 self.window.push_back(lower);
//             }
//             self.window.pop_front()
//         } else {
//             None
//         }
//     }
// }
