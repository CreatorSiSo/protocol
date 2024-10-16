use std::{
    collections::VecDeque,
    fmt::Debug,
    io::{stdin, Read}, thread, time::Duration,
};

mod device;
use device::{B15fDevice, Device};

fn main() -> Result<(), &'static str> {
    let mut connection = Connection::new(B15fDevice::new()?);

    let input = stdin().lock().bytes();
    connection
        .outgoing_data
        .extend(input.flat_map(|maybe_byte| maybe_byte.ok()));

    for i in 0..100 {
        thread::sleep(Duration::from_millis(100));
        connection.poll();
    }

    dbg!(String::from_utf8_lossy(&connection.incoming_data));
    Ok(())
}

/// Start of frame
const SOF: u8 = 0x66;

/// End of frame
const EOF: u8 = 0x77;

/// Correct frame data
const CFD: u8 = 0x88;

/// Incorrect frame data
const IFD: u8 = 0x99;

const ESCAPE_CODES: [u8; 4] = [SOF, EOF, CFD, IFD];

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
/// These values are encoded by repeating the value once.
///
/// | Function             | Escape code | Escaped value |
/// | -------------------- | ----------- | ------------- |
/// | start of frame       | (SOF) 0x66  | 0x6666        |
/// | end of frame         | (EOF) 0x77  | 0x7777        |
/// | correct frame data   | (CDF) 0x88  | 0x8888        |
/// | incorrect frame data | (IDF) 0x99  | 0x9999        |
///
fn encode_frame<const S: usize>(data: &[u8; S]) -> Box<[u8]> {
    let sof_len = 1;
    let eof_len = 1;
    let checksum_len = 0;
    let frame_len = sof_len + data.len() + checksum_len + eof_len;

    let mut frame = Vec::with_capacity(frame_len);
    frame.push(SOF);

    for value in data {
        if ESCAPE_CODES.contains(value) {
            frame.push(*value);
        }
        frame.push(*value);
    }

    // TODO Encode chucksums

    frame.push(EOF);

    frame.into_boxed_slice()
}

/// TODO Maybe break this up?
///
/// 1. pull data out of frame
/// 2. unescape values
/// 3. calculate checksums for received data
/// 4. compare checksums
///
fn decode_frame() {}

enum IncomingStatus {
    WaitingForStart,
    Receiving,
}

#[derive(PartialEq)]
enum OutgoingStatus {
    Sending,
    Finished,
}

#[derive(Default)]
struct Connection<D: Device> {
    incoming_data: Vec<u8>,
    outgoing_data: VecDeque<u8>,

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

impl<D: Device> Connection<D> {
    fn new(device: D) -> Self {
        Self {
            incoming_data: Vec::new(),
            outgoing_data: VecDeque::new(),
            incoming: 0,
            outgoing: 0,
            device
        }
    }

    fn poll(&mut self) {
        // TODO Synchronization

        if let Some(byte) = self.outgoing_data.pop_front() {
            println!("<- {:?}", byte);
            self.send(byte);
        }

        let received = self.receive();
        println!("-> {:?}\n", received);

        if let Some(byte) = received {
            self.incoming_data.push(byte);
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

    fn send(&mut self, data: u8) {
        println!("byte: {:08b}", data);

        const MASK: u8 = 0x0f;
        let manchester_coded_1 = data ^ MASK;
        let manchester_coded_2 = data ^ !MASK;

        self.device.send(manchester_coded_1 >> 4);
        println!("{:?}", &self);
        self.device.send(manchester_coded_2 >> 4);
        println!("{:?}", &self);

        self.device.send(manchester_coded_1 & 0x0f);
        println!("{:?}", &self);
        self.device.send(manchester_coded_2 & 0x0f);
        println!("{:?}", &self);
    }
}

impl<D: Device> Debug for Connection<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04b}", self.outgoing)?;
        Ok(())
    }
}
