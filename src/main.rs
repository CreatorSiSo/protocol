use std::io::{stdin, stdout, Read, Write};
use std::{thread, time::Duration};

mod transport;

mod device;
use device::{B15fDevice, Device};
use escape::{EscapeCode, Escaped};

mod escape;

mod stream;
use stream::{Command, InputStream, OutputStream};

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let stdout = stdout().lock();
    // let input = fs::read("./data/random-256.bin").unwrap();

    let mut connection = Connection::new(stdin, std::iter::empty(), stdout);

    while connection.poll() {
        thread::sleep(Duration::from_millis(1));
    }

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

trait Bytes: Iterator<Item = std::io::Result<u8>> {}
impl<T: Iterator<Item = std::io::Result<u8>>> Bytes for T {}

struct Connection<D: Bytes, R: Bytes, W: Write> {
    data: Escaped<D>,
    input: R,
    output: W,
    clock: usize,
    done_receiving: bool,
}

impl<D: Bytes, R: Bytes, W: Write> Connection<D, R, W> {
    fn new(data: D, input: R, output: W) -> Self {
        Self {
            data: Escaped::new(data),
            input,
            output,
            clock: 0,
            done_receiving: false,
        }
    }

    // Returns false when all data has been sent and received
    fn poll(&mut self) -> bool {
        // Read byte and decode command
        let byte_in = self.input.next();
        eprintln!("input: {:0x?}", byte_in);
        // match self.i_stream.push(nibble_in) {
        //     Command::Received(frame) => output.write_all(decode_frame(&frame)).unwrap(),
        //     Command::SendNextFrame => {
        //         for line in &mut self.debug_lines {
        //             eprintln!("{} {}", self.device.name(), line);
        //             line.clear();
        //         }
        //         self.o_stream.send_frame(encode_frame(&mut self.data));
        //     }
        //     Command::ResendLastFrame => self.o_stream.resend_frame(),
        //     Command::StopReceivingData => self.done_receiving = true,
        //     Command::None => (),
        // };

        // Write byte
        let byte_out = self
            .data
            .next()
            .unwrap_or(Ok((self.clock % 2) as u8))
            .unwrap();
        self.output.write_all(&[byte_out]).unwrap();
        eprintln!("out:   {:0x?}", byte_out);

        !(self.data.is_done() && self.done_receiving)
    }
}
