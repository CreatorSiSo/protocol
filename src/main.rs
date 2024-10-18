use std::io::{stdin, stdout, Bytes, Read, Write};
use std::{thread, time::Duration};

mod device;
use device::{B15fDevice, Device, FileDevice};
use escape::{EscapeCode, EscapedBytes};

mod escape;

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let mut connection = Connection::new(FileDevice::new(), stdin);

    for _ in 0..500 {
        thread::sleep(Duration::from_millis(1));
        connection.poll();
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
/// | negate previous nibble | (NPN) 0x56  | 0x56 0x65      |
/// | done sending           | (DS)  0x67  | 0x67 0x76      |
///
/// 0x56 0x65 0x9a 0x56
/// 0x56      0x9a 0x56
/// 0x56      0x65
///
fn encode_frame(data: &mut EscapedBytes<impl Read>) -> Frame {
    let mut frame = [0; FRAME_LEN];
    frame[0] = EscapeCode::SOF as u8;

    for cell in &mut frame[1..(1 + FRAME_DATA_LEN)] {
        *cell = match data.next() {
            Some(Ok(byte)) => byte,
            Some(Err(err)) => todo!("{}", err),
            // TODO Send finished escape code
            None => break,
        }
    }

    // TODO Encode chucksums

    frame[FRAME_LEN - 1] = EscapeCode::EOF as u8;

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
    device: D,
}

impl<D: Device, R: Read> Connection<D, R> {
    fn new(device: D, bytes: Bytes<R>) -> Self {
        let mut data = EscapedBytes::new(bytes);
        Self {
            o_stream: OutputStream::new(encode_frame(&mut data)),
            i_stream: InputStream::new(),
            data,
            device,
        }
    }

    fn poll(&mut self) {
        if let Some(nibble_out) = self.o_stream.pull() {
            self.next_frame = encode_frame(&mut self.data);
            self.sending_index = 0;
            println!("{:?}", self.next_frame);
        }

        let byte = self.next_frame[self.sending_index / 2];
        // println!("byte: {:08b}", byte);
        if self.sending_index % 2 == 0 {
            self.device.send(nibble_out);
        } else {
            self.device.send(byte & 0x0f)
            self.o_stream = OutputStream::new(encode_frame(&mut self.data));
        };

        let nibble_in = self.device.read();
        // println!("-> {:?}", received);
        match self.i_stream.push(nibble_in) {
            Input::Frame(frame) => stdout().lock().write_all(decode_frame(&frame)).unwrap(),
            Input::Finished => todo!(),
            Input::Receiving => (),
        }
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

    fn push(&mut self, nibble: u8) -> Input {
        Input::Receiving
    }
}

enum Input {
    Frame(Frame),
    Receiving,
    Finished,
}

struct OutputStream {
    /// Data to send
    frame: Frame,
    /// Index of the nibble to send
    index: usize,
}

impl OutputStream {
    fn new(frame: Frame) -> Self {
        println!("{:?}", frame);
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
}
