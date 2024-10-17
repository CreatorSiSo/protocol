use std::{
    fmt::Debug,
    io::{stdin, Bytes, Read},
    thread,
    time::Duration,
};

mod device;
use device::{B15fDevice, Device, FileDevice};
use escape::{EscapeCode, EscapedBytes};

mod escape;

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let mut connection = Connection::new(B15fDevice::new()?, stdin);

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(10));
        connection.poll();
    }

    dbg!(String::from_utf8_lossy(&connection.received));
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

/// TODO Maybe break this up?
///
/// 1. pull data out of frame
/// 2. unescape values
/// 3. calculate checksums for received data
/// 4. compare checksums
///
fn decode_frame() {}

struct Connection<D: Device, R: Read> {
    data: EscapedBytes<R>,
    next_frame: Frame,
    // Index of nibble to send
    sending_index: usize,

    received: Vec<u8>,
    received_frame: Frame,

    /// last nibble received
    /// - upper nibble: unused
    /// - lower nibble: data received
    incoming: u8,
    /// last nibble sent
    /// - upper nibble: unused
    /// - lower nibble: data sent
    outgoing: u8,

    device: D,
}

impl<D: Device, R: Read> Connection<D, R> {
    fn new(device: D, data: Bytes<R>) -> Self {
        Self {
            data: EscapedBytes::new(data),
            next_frame: [0; FRAME_LEN],
            sending_index: 0,

            received: Vec::new(),
            received_frame: [0; FRAME_LEN],

            incoming: 0,
            outgoing: 0,
            device,
        }
    }

    fn poll(&mut self) {
        if self.sending_index == 0 || self.sending_index >= (FRAME_LEN * 2) {
            self.next_frame = encode_frame(&mut self.data);
            self.sending_index = 0;
            println!("{:?}", self.next_frame);
        }

        let byte = self.next_frame[self.sending_index / 2];
        // println!("byte: {:08b}", byte);
        if self.sending_index % 2 == 0 {
            self.device.send(byte >> 4);
        } else {
            self.device.send(byte & 0x0f)
        };
        self.sending_index += 1;

        let received = self.receive();
        // println!("-> {:?}", received);

        if let Some(byte) = received {
            self.received.push(byte);
        }
    }

    fn receive(&mut self) -> Option<u8> {
        let previous = self.incoming & 0x0f;
        let current = self.device.read();

        // update input if different
        if previous == current {
            return None;
        }

        // decode Manchester code
        //
        // | previous | current | byte |
        // | -------- | ------- | ---- |
        // | 0        | 0       | x    |
        // | 0        | 1       | 1    |
        // | 1        | 0       | 0    |
        // | 1        | 1       | x    |
        let byte = !previous & current;

        return Some(byte);
    }
}

impl<D: Device, R: Read> Debug for Connection<D, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04b}", self.outgoing)?;
        Ok(())
    }
}
