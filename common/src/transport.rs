use crate::bitvec::BitVec;
use crate::device::Device;

// #[cfg(target_arch = "avr")]
// macro_rules! dbg {
//     () => {};
//     ($val:expr $(,)?) => {
//         match $val {
//             val => val,
//         }
//     };
//     ($($val:expr),+ $(,)?) => {
//         $val
//     };
// }

// How many bits are sent at once
const BIT_WIDTH: usize = 3;

pub struct TransportEncode<D: Device> {
    device: D,
    clock: BitVec<1>,
    data: BitVec<64>,
}

impl<D: Device> TransportEncode<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            clock: BitVec::from_byte(0, 4),
            data: BitVec::new(),
        }
    }

    pub fn poll(&mut self) {
        // Concat clock and data into the nibble to be sent
        let Some(data) = self.data.pop_back::<1>(BIT_WIDTH) else {
            return;
        };
        let mut nibble = self.clock.clone();
        nibble.push_back(&data);

        self.device.write(nibble);

        if self.clock.get(0).unwrap() == false {
            self.clock.set(0, true);
        } else {
            self.clock.set(0, false);
        }
    }

    pub fn push(&mut self, byte: u8) {
        self.data.push_front(&BitVec::<1>::from_byte(byte, 8));
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

    let mut encoder = TransportEncode::new(TestDevice {});
    encoder.push(0xff);
    encoder.poll();
    encoder.poll();
    for byte in [0xf0; 4] {
        encoder.push(byte);
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

pub struct TransportDecode<D: Device> {
    device: D,
    last_clock: bool,
    data: BitVec<64>,
}

impl<D: Device> TransportDecode<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            last_clock: false,
            data: BitVec::new(),
        }
    }

    // Tries to read data from the cable
    pub fn poll(&mut self) {
        let mut nibble = self.device.read();
        let next_clock = nibble.pop_front::<1>(1).unwrap().get(0).unwrap();
        if next_clock == self.last_clock {
            // Clock has not changed since last read
            return;
        }
        self.last_clock = next_clock;
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
                BitVec::from_byte(0b0001_0000, 4)
            } else {
                BitVec::from_byte(0b1000_0000, 4)
            }
        }

        fn write(&mut self, _data: BitVec<1>) {
            unimplemented!()
        }
    }

    let mut decoder = TransportDecode::new(TestDevice { clock: true });
    decoder.poll();
    decoder.poll();
    assert_eq!(decoder.data.len(), 6);
    assert_eq!(decoder.read(), None);

    decoder.poll();
    // assert_eq!(decoder.read(), Some(4));
    // assert_eq!(decoder.data.len, 1);
}
