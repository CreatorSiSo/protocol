use crate::bititer::Byte;
use crate::bitvec::BitVec;
use crate::device::Device;
use crate::escape::EscapeCode;
use crate::{Frame, FRAME_LEN};

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

// How many bits are sent at once
const BIT_WIDTH: usize = 3;
const BUFFER_SIZE: usize = FRAME_LEN * 3;

pub struct Encoder {
    clock: bool,
    data: BitVec<BUFFER_SIZE>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            clock: false,
            data: BitVec::new(),
        }
    }

    pub fn poll(&mut self, device: &mut impl Device) {
        // #[cfg(target_arch = "avr")]
        // ufmt::uwriteln!(device.serial(), "encode {}\r", self.data).unwrap();

        // Concat clock and data into the nibble to be sent
        let Some(mut data) = self.data.pop_front::<1>(BIT_WIDTH) else {
            return;
        };
        data.push_front(&self.clock);

        device.write(data);

        self.clock = !self.clock;
    }

    pub fn send_sync_req(&mut self) {
        self.send_bytes([EscapeCode::SyncReq as u8, EscapeCode::Noop as u8])
    }

    pub fn send_sync_res(&mut self) {
        self.send_bytes([EscapeCode::SyncRes as u8, EscapeCode::Noop as u8])
    }

    pub fn send_frame(&mut self, frame: &Frame) {
        self.send_bytes(frame.encode())
    }

    pub fn send_ack(&mut self, index: u8) {
        self.send_bytes([EscapeCode::Ack as u8, index, EscapeCode::Noop as u8])
    }

    pub fn send_nack(&mut self, index: u8) {
        self.send_bytes([EscapeCode::Nack as u8, index, EscapeCode::Noop as u8])
    }

    pub fn send_finished(&mut self) {
        self.send_bytes([EscapeCode::Finished as u8, EscapeCode::Noop as u8])
    }

    pub fn send_noop(&mut self) {
        self.send_bytes([EscapeCode::Noop as u8])
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn needs_data(&self) -> bool {
        self.data.len() < 3
    }

    fn send_bytes<const N: usize>(&mut self, bytes: [u8; N]) {
        let bitvec = BitVec::from_bytes(bytes, bytes.len() * 8);
        if self.data.push_back(&bitvec) {
            panic!("encoder full!");
        }
    }
}

#[test]
fn encode() {
    struct TestDevice {}
    impl Device for TestDevice {
        fn read(&mut self) -> BitVec<1> {
            unimplemented!()
        }

        fn write(&mut self, _data: BitVec<1>) {}
    }
    let mut device = TestDevice {};

    let mut encoder = Encoder::new();
    encoder.send_sync_req();
    assert_eq!(
        encoder.data,
        BitVec::from_bytes([EscapeCode::SyncReq as u8], 8),
    );
    encoder.poll(&mut device);
    encoder.poll(&mut device);

    // Should not do anything
    {
        encoder.poll(&mut device);
        encoder.poll(&mut device);
        encoder.poll(&mut device);
    }

    assert_eq!(encoder.data, BitVec::from_bytes([0b0100_0000], 2));

    encoder.send_ack(99);
    for _ in 0..(2 + 16) {
        encoder.poll(&mut device);
    }
    assert_eq!(encoder.data, BitVec::from_bytes([], 0));

    dbg!(&encoder.data);
}

pub struct Decoder {
    last: BitVec<1>,
    data: BitVec<BUFFER_SIZE>,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            last: BitVec::from(&Byte(0x00)),
            data: BitVec::new(),
        }
    }

    // Tries to read data and decode from the cable
    pub fn poll(&mut self, device: &mut impl Device) -> Command {
        let mut next = device.read();
        if next == self.last {
            // Clock has not changed since last read
            return Command::None;
        }
        // dbg!(next);
        // #[cfg(target_arch = "avr")]
        // ufmt::uwriteln!(device.serial(), "nibble {}\r", next).unwrap();

        self.last = next;
        next.shrink_front(1);
        self.data.push_back(&next);

        for (code, bitvec) in EscapeCode::all() {
            let Some(width) = self.data.find(&bitvec) else {
                continue;
            };
            self.data.shrink_front(width);

            let mut decode_with_number = || {
                (self.data.len() >= (8 + 8)).then(|| {
                    self.data.shrink_front(8);
                    self.data.pop_front::<1>(8).unwrap().bytes()[0]
                })
            };

            let maybe_command = match code {
                EscapeCode::SyncReq => {
                    self.data.shrink_front(8);
                    Some(Command::SyncReq)
                }
                EscapeCode::SyncRes => {
                    self.data.shrink_front(8);
                    Some(Command::SyncRes)
                }
                EscapeCode::Noop => {
                    self.data.shrink_front(8);
                    None
                }

                EscapeCode::StartOfFrame => self
                    .data
                    .pop_front::<FRAME_LEN>(FRAME_LEN * 8)
                    .map(|bits| Command::Frame(Frame::decode(bits.bytes()))),

                EscapeCode::Ack => {
                    self.data.shrink_front(8);
                    Some(Command::Ack)
                }
                EscapeCode::Nack => {
                    self.data.shrink_front(8);
                    Some(Command::Nack)
                }
                EscapeCode::Finished => {
                    decode_with_number().map(|len| Command::Finished /* (len) */)
                }
            };

            if let Some(command) = maybe_command {
                return command;
            }
        }

        Command::None
    }
}

#[test]
fn decode() {
    struct TestDevice {
        clock: bool,
        data: BitVec<512>,
    }

    impl Device for TestDevice {
        fn read(&mut self) -> BitVec<1> {
            let mut nibble = BitVec::new();

            nibble.push_back(&self.clock);
            self.clock = !self.clock;

            nibble.push_back(&if let Some(data) = self.data.pop_front::<1>(3) {
                data
            } else {
                BitVec::from(&[false, false, false])
            });

            nibble
        }

        fn write(&mut self, _data: BitVec<1>) {
            unimplemented!()
        }
    }

    let mut data = BitVec::new();
    data.push_back(&BitVec::from_bytes(
        [0, EscapeCode::SyncReq as u8, 0],
        3 * 8,
    ));
    data.push_back(&BitVec::from_bytes(
        Frame::empty_invalid().encode(),
        FRAME_LEN * 8,
    ));

    let mut device = TestDevice { clock: true, data };

    let mut decoder = Decoder::new();
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(dbg!(decoder.data).len(), 9);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.data.len(), 15);
    assert_eq!(decoder.poll(&mut device), Command::SyncReq);
    assert_eq!(decoder.data.len(), 2);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.poll(&mut device), Command::None);
    assert_eq!(decoder.data.len(), 8);
    for _ in 0..213 {
        assert_eq!(decoder.poll(&mut device), Command::None);
    }
    assert_eq!(
        decoder.poll(&mut device),
        Command::Frame(Frame::empty_invalid())
    );
}

#[derive(Debug, ufmt::derive::uDebug, PartialEq, Eq)]
pub enum Command {
    SyncReq,
    SyncRes,
    Frame(Frame),
    Ack,
    Nack,
    Finished,
    None,
}
