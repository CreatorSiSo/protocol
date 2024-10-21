use std::io::{stdin, stdout, Read, Write};
use std::{thread, time::Duration};

mod device;
use device::{DebugDevice, Device};
use escape::{swap_nibbles, EscapeCode, Escaped};

mod escape;

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let mut connection = Connection::new(DebugDevice::new(), stdin);

    while connection.poll() {
        thread::sleep(Duration::from_millis(1));
    }

    // dbg!(String::from_utf8_lossy(&connection.received));
    Ok(())
}

const ESCAPE_CODE_LEN: usize = 1;
const CHECKSUM_LEN: usize = 0;
const FRAME_DATA_LEN: usize = 64;
const FRAME_LEN: usize = ESCAPE_CODE_LEN + FRAME_DATA_LEN + CHECKSUM_LEN + ESCAPE_CODE_LEN;
type Frame = [u8; FRAME_LEN];

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
/// | start of frame         | (SOF) 0x12  | 0x12 0x21      |
/// | end of frame           | (EOF) 0x23  | 0x23 0x32      |
/// | correct frame data     | (CDF) 0x34  | 0x34 0x43      |
/// | incorrect frame data   | (IDF) 0x45  | 0x45 0x54      |
/// | buffer                 | (BU) 0x56   | 0x56 0x65      |
/// | finished sending       | (FS)  0x67  | 0x67 0x76      |
///
/// 0x56 0x65 0x9a 0x56
/// 0x56      0x9a 0x56
/// 0x56      0x65
///
fn encode_frame(data: &mut impl Iterator<Item = std::io::Result<u8>>) -> Frame {
    let mut frame = [0; FRAME_LEN];
    frame[0] = EscapeCode::StartOfFrame as u8;

    for cell in &mut frame[1..(1 + FRAME_DATA_LEN)] {
        *cell = match data.next() {
            Some(Ok(byte)) => byte,
            Some(Err(err)) => todo!("{}", err),
            // TODO Send finished escape code
            None => break,
        }
    }

    // TODO Encode chucksums

    frame[FRAME_LEN - 1] = EscapeCode::EndOfFrame as u8;

    frame
}

/// 1. pull data out of frame
/// 2. calculate checksums for received data
/// 3. compare checksums
///
fn decode_frame(frame: &Frame) -> &[u8] {
    &frame[1..(1 + FRAME_DATA_LEN)]
}

struct Connection<D: Device, I: Iterator<Item = std::io::Result<u8>>> {
    i_stream: InputStream,
    o_stream: OutputStream,
    data: Escaped<I>,
    done_receiving: bool,
    device: D,
    debug_lines: [String; 4],
}

impl<D: Device, I: Iterator<Item = std::io::Result<u8>>> Connection<D, I> {
    fn new(device: D, bytes: I) -> Self {
        let mut data = Escaped::new(bytes);
        Self {
            o_stream: OutputStream::new(encode_frame(&mut data)),
            i_stream: InputStream::new(),
            data,
            done_receiving: false,
            device,
            debug_lines: [const { String::new() }; 4],
        }
    }

    // Returns false when all data has been sent and received
    fn poll(&mut self) -> bool {
        if let Some(nibble_out) = self.o_stream.pull() {
            for i in 0..4 {
                let block = if (nibble_out << i) & 0b1000 == 0b1000 {
                    "◻️"
                } else {
                    "◼"
                };
                self.debug_lines[i].push_str(block);
            }
            self.device.send(nibble_out);
        };

        let nibble_in = self.device.read();
        // println!("-> {:?}", received);
        match self.i_stream.push(nibble_in) {
            Command::Received(frame) => stdout().lock().write_all(decode_frame(&frame)).unwrap(),
            Command::SendNextFrame => {
                for line in &mut self.debug_lines {
                    eprintln!("{} {}", self.device.name(), line);
                    line.clear();
                }
                self.o_stream = OutputStream::new(encode_frame(&mut self.data));
            }
            Command::ResendLastFrame => self.o_stream.reset(),
            Command::StopReceivingData => self.done_receiving = true,
            Command::None => (),
        };

        self.device.debug_poll();

        !(self.data.is_done() && self.done_receiving)
    }
}

struct InputStream {
    // the last 4 nibbles that have been received
    window: u16,
    // how many nibbles have been pushed into the window
    window_length: u8,
    frame: Frame,
    // index of byte in the frame to write to next
    frame_index: usize,
}

impl InputStream {
    fn new() -> Self {
        Self {
            window: 0x0000,
            window_length: 0,
            frame: [0; FRAME_LEN],
            frame_index: 0,
        }
    }

    fn push(&mut self, nibble: u8) -> Command {
        // ensures that the unused nibble is 0
        let nibble = nibble & 0x0f;
        // truncates the u16, so that only the least significant nibble is left
        let previous_nibble = (self.window as u8) & 0x0f;
        // whether value on cable has changed
        if previous_nibble == nibble {
            return Command::None;
        }

        // push received nibble
        self.window <<= 4;
        self.window |= nibble as u16;
        self.window_length += 1;
        // not enough data has been pushed into the window
        if self.window_length < 4 {
            return Command::None;
        }

        let higher_byte = (self.window >> 4) as u8;
        let lower_byte = self.window as u8;

        // detect escape codes and shrink the window,
        // so that the data is not decoded again in the next iteration
        let command = match EscapeCode::from_byte(higher_byte) {
            None => {
                self.window_length = 2;
                Command::None
            }
            Some(_) if higher_byte == swap_nibbles(lower_byte) => {
                self.window_length = 0;
                Command::None
            }
            Some(escape_code) => {
                self.window_length = 2;

                match escape_code {
                    EscapeCode::StartOfFrame if self.frame_index != 0 => Command::ResendLastFrame,
                    EscapeCode::EndOfFrame if self.frame_index != (self.frame.len() - 1) => {
                        Command::ResendLastFrame
                    }
                    EscapeCode::StartOfFrame => Command::None,
                    EscapeCode::EndOfFrame => {
                        self.frame[FRAME_LEN - 1] = EscapeCode::EndOfFrame as u8;
                        self.frame_index = 0;
                        Command::Received(self.frame)
                    }
                    EscapeCode::Buffer => todo!(),
                    EscapeCode::CorrectFrameData => Command::SendNextFrame,
                    EscapeCode::IncorrectFrameData => Command::ResendLastFrame,
                    EscapeCode::FinishedSending => Command::StopReceivingData,
                }
            }
        };

        // received part of a frame not an inserted escape code
        if command == Command::None {
            eprintln!("{:?}", self.frame);
            self.frame[self.frame_index] = higher_byte;
            self.frame_index += 1;
        }

        command
    }
}

#[derive(PartialEq, Eq)]
enum Command {
    Received(Frame),
    SendNextFrame,
    ResendLastFrame,
    /// From now on the other side will only send escape codes
    StopReceivingData,
    None,
}

struct OutputStream {
    /// Data to send
    frame: Frame,
    /// Index of the nibble to send
    index: usize,
}

impl OutputStream {
    fn new(frame: Frame) -> Self {
        eprintln!("{:?}", frame);
        Self { frame, index: 0 }
    }

    /// returns the next nibble to send
    fn pull(&mut self) -> Option<u8> {
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
    fn reset(&mut self) {
        self.index = 0;
    }
}
