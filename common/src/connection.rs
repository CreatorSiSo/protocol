use crate::{transport::TransportDecode, Device, TransportEncode, CHECKSUM_LEN, FRAME_DATA_LEN};

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

pub trait Connection {
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

pub struct MirrorConnection<D: Device> {
    output_state: OutputState,
    encoder: TransportEncode,
    input_state: InputState,
    decoder: TransportDecode,
    device: D,
    bytes_sent: u32,
}

impl<D: Device> MirrorConnection<D> {
    pub fn new(device: D) -> Self {
        Self {
            output_state: OutputState::WaitingForFrame,
            encoder: TransportEncode::new(),
            input_state: InputState::WaitingForConnection,
            decoder: TransportDecode::new(),
            device,
            bytes_sent: 0,
        }
    }

    pub fn poll(&mut self) {
        let to_be_sent = if self.input_state == InputState::WaitingForConnection {
            Some(if (self.bytes_sent % 2) == 0 {
                0xff
            } else {
                0x00
            })
        } else {
            None
        };

        if let Some(byte) = to_be_sent {
            self.encoder.push(byte);
            self.bytes_sent = self.bytes_sent.wrapping_add(1);
        }

        self.encoder.poll(&mut self.device);
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
