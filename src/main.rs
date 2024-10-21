use std::io::{stdin, stdout, Read, Write};
use std::{thread, time::Duration};

mod device;
use device::{DebugDevice, Device};
use escape::{EscapeCode, Escaped};

mod escape;

mod stream;
use stream::{Command, InputStream, OutputStream};

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

/// 1. calculate checksums for received data
/// 2. compare checksums
///
fn decode_frame(frame: &[u8; FRAME_DATA_LEN + CHECKSUM_LEN]) -> &[u8] {
    frame
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
