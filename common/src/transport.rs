use crate::bititer::Byte;
use crate::bitvec::BitVec;
use crate::device::Device;

// How many bits are sent at once
const BIT_WIDTH: usize = 3;

pub struct TransportEncode {
    clock: bool,
    data: BitVec<64>,
}

impl TransportEncode {
    pub fn new() -> Self {
        Self {
            clock: false,
            data: BitVec::new(),
        }
    }

    pub fn establish_connection(&mut self, device: &mut impl Device) {
        let nibble = BitVec::from(&Byte(if self.clock { 0xf0 } else { 0x00 }));
        device.write(nibble);
        self.clock = !self.clock;
    }

    pub fn poll(&mut self, device: &mut impl Device) {
        // Concat clock and data into the nibble to be sent
        let Some(mut data) = self.data.pop_back::<1>(BIT_WIDTH) else {
            return;
        };
        data.push_front(&self.clock);

        device.write(data);

        self.clock = !self.clock;
    }

    pub fn push(&mut self, byte: &BitVec<1>) {
        self.data.push_front(byte)
    }

    pub fn amount_bits_remaining(&self) -> usize {
        self.data.len()
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

    let mut encoder = TransportEncode::new();
    encoder.push(&BitVec::from(&Byte(0xff)));
    encoder.poll(&mut device);
    encoder.poll(&mut device);
    dbg!(&encoder.data);
    for byte in [0xf0; 4] {
        encoder.push(&BitVec::from(&Byte(byte)));
        dbg!(&encoder.data);
    }
    assert_eq!(
        BitVec::from_bytes(
            [
                0b1111_0000,
                0b1111_0000,
                0b1111_0000,
                0b1111_0000,
                0b1100_0000
            ],
            34
        ),
        encoder.data
    );
}

pub struct TransportDecode {
    last: BitVec<1>,
    data: BitVec<64>,
}

impl TransportDecode {
    pub fn new() -> Self {
        Self {
            last: BitVec::from(&Byte(0x00)),
            data: BitVec::new(),
        }
    }

    pub fn establish_connection(&mut self, device: &mut impl Device) -> bool {
        let mut nibble = device.read();
        let next = nibble.pop_front::<1>(1).unwrap();
        let successful = !next == self.last;
        self.last = next;
        successful
    }

    // Tries to read data from the cable
    pub fn poll(&mut self, device: &mut impl Device) {
        let mut nibble = device.read();
        let next = nibble.pop_front::<1>(1).unwrap();
        if next.get(0).unwrap() == self.last.get(0).unwrap() {
            // Clock has not changed since last read
            return;
        }
        self.last = next;
        self.data.push_back(&nibble);
    }

    // Returns the next full byte of received data,
    // `None` if no full 8 bits of data are available (yet)
    pub fn read(&mut self) -> Option<BitVec<1>> {
        self.data.pop_front::<1>(8)
    }
}

#[test]
fn decode() {
    struct TestDevice {
        clock: bool,
    }
    impl Device for TestDevice {
        fn read(&mut self) -> BitVec<1> {
            self.clock = !self.clock;
            if self.clock {
                BitVec::from(&[false, false, false, true])
            } else {
                BitVec::from(&[true, false, false, false])
            }
        }

        fn write(&mut self, _data: BitVec<1>) {
            unimplemented!()
        }
    }
    let mut device = TestDevice { clock: true };

    let mut decoder = TransportDecode::new();
    decoder.poll(&mut device);
    decoder.poll(&mut device);
    assert_eq!(decoder.data.len(), 6);
    assert_eq!(decoder.read(), None);

    decoder.poll(&mut device);
    assert_eq!(decoder.read(), Some(BitVec::from(&Byte(0b000_001_00))));
    assert_eq!(decoder.data.len(), 1);
}
