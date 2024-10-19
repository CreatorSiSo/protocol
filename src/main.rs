use std::io::{stdin, stdout, Bytes, Read, Write};
use std::{thread, time::Duration};

mod device;
use device::{B15fDevice, Device, FileDevice};
use escape::{EscapeCode, EscapedBytes};

mod escape;

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let mut connection = Connection::new(FileDevice::new(), stdin);

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
/// | negate following       | (NF) 0x56   | 0x56 0x65      |
/// | finished sending       | (FS)  0x67  | 0x67 0x76      |
///
/// 0x56 0x65 0x9a 0x56
/// 0x56      0x9a 0x56
/// 0x56      0x65
///
fn encode_frame(data: &mut EscapedBytes<impl Read>) -> Frame {
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
/// 2. unescape values
/// 3. calculate checksums for received data
/// 4. compare checksums
///
fn decode_frame(frame: &Frame) -> &[u8] {
    frame
}

struct Connection<D: Device, R: Read> {
    i_stream: InputStream,
    o_stream: OutputStream,
    data: EscapedBytes<R>,
    done_receiving: bool,
    device: D,
    debug_lines: [String; 4],
}

impl<D: Device, R: Read> Connection<D, R> {
    fn new(device: D, bytes: Bytes<R>) -> Self {
        let mut data = EscapedBytes::new(bytes);
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
                    eprintln!("{}", line);
                    line.clear();
                }
                self.o_stream = OutputStream::new(encode_frame(&mut self.data));
            }
            Command::ResendLastFrame => self.o_stream.reset(),
            Command::StopReceivingData => self.done_receiving = true,
            Command::None => (),
        };

        !(self.data.is_done() && self.done_receiving)
    }
}

struct InputStream {
    incoming: u16,
    frame: Frame,
    index: usize,
}

impl InputStream {
    fn new() -> Self {
        Self {
            incoming: 0x0000,
            frame: [0; FRAME_LEN],
            index: 0,
        }
    }

    fn push(&mut self, nibble: u8) -> Command {
        // truncates the u16, so that only the least significant byte is left
        let previous_byte = self.incoming as u8;

        if (previous_byte & 0x0f) == (nibble & 0x0f) {
            // Values on cable have not changed
            return Command::None;
        }

        self.incoming <<= 4;
        self.incoming |= (nibble & 0x0f) as u16;

        let current_byte = self.incoming as u8;
        match EscapeCode::from_byte(current_byte) {
            Some(EscapeCode::StartOfFrame) if self.index != 0 => Command::ResendLastFrame,
            Some(EscapeCode::EndOfFrame) if self.index != (self.frame.len() - 1) => {
                Command::ResendLastFrame
            }
            Some(EscapeCode::NegateFollowing) => todo!(),
            Some(EscapeCode::CorrectFrameData) => Command::SendNextFrame,
            Some(EscapeCode::IncorrectFrameData) => Command::ResendLastFrame,
            Some(EscapeCode::FinishedSending) => Command::StopReceivingData,
            _ => Command::None,
        }

        // TODO Push received data
    }
}

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
