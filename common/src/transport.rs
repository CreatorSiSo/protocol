use core::fmt::Debug;

use bitvec::{
    array::BitArray,
    order::{BitOrder, Msb0},
    slice::BitSlice,
    store::BitStore,
    view::BitView,
};

use crate::device::Device;

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

pub struct TransportEncode<D: Device> {
    device: D,
    clock: u8,
    data: BoundedBitVec<u8, 64, Msb0>,
}

impl<D: Device> TransportEncode<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            clock: 0,
            data: BoundedBitVec::new(),
        }
    }

    pub fn poll(&mut self) {
        let nibble = self.clock | self.pop(BIT_WIDTH);
        self.device.write(nibble);
        self.clock = if self.clock == 0 { 0b1000 } else { 0 }
    }

    pub fn push(&mut self, byte: u8) {
        self.data.push_front(byte.view_bits());
    }

    fn pop(&mut self, width: usize) -> u8 {
        const fn bitmask_lower(width: usize) -> u8 {
            (1 << width) - 1
        }

        self.data.pop_back(width) & dbg!(bitmask_lower(width))
    }
}

#[test]
fn encode() {
    use bitvec::{bitarr, order::Lsb0};

    struct TestDevice {}
    impl Device for TestDevice {
        fn read(&mut self) -> u8 {
            unimplemented!()
        }

        fn write(&mut self, _data: u8) {}
    }

    let mut encoder = TransportEncode::new(TestDevice {});
    encoder.push(0xff);
    encoder.poll();
    encoder.poll();
    for byte in [0xf0; 4] {
        encoder.push(byte);
    }
    assert_eq!(
        bitarr!(
            1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0,
            0, 0, 0, 1, 1
        )[..34],
        encoder.data.as_bitslice()
    );
}

pub struct TransportDecode<D: Device> {
    device: D,
    last_clock: u8,
    data: BoundedBitVec<u8, 64, Msb0>,
}

impl<D: Device> TransportDecode<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            last_clock: 0,
            data: BoundedBitVec::new(),
        }
    }

    // Tries to read data from the cable
    pub fn poll(&mut self) {
        let nibble = self.device.read();
        let next_clock = (nibble & 0xf) >> 3;
        if next_clock == self.last_clock {
            // Clock has not changed since last read
            return;
        }
        self.last_clock = next_clock;
        self.data.push_back(&nibble.view_bits()[5..8]);
    }

    // Returns the next full byte of received data,
    // `None` if no full 8 bits of data are available (yet)
    pub fn read(&mut self) -> Option<u8> {
        (self.data.len >= 8).then(|| self.data.pop_front(8))
    }
}

#[test]
fn decode() {
    struct TestDevice {
        clock: u8,
    }

    impl Device for TestDevice {
        fn read(&mut self) -> u8 {
            self.clock = !self.clock & 0b1;
            if self.clock == 1 {
                0b0001
            } else {
                0b1000
            }
        }

        fn write(&mut self, _data: u8) {
            unimplemented!()
        }
    }

    let mut decoder = TransportDecode::new(TestDevice { clock: 1 });
    decoder.poll();
    decoder.poll();
    assert_eq!(decoder.read(), None);
    assert_eq!(decoder.data.len, 6);

    decoder.poll();
    assert_eq!(decoder.read(), Some(4));
    assert_eq!(decoder.data.len, 1);
}

struct BoundedBitVec<T: BitStore, const C: usize, O: BitOrder> {
    inner: BitArray<[T; C], O>,
    len: usize,
}

impl<T: BitStore + Copy + From<u8>, const C: usize, O: BitOrder> BoundedBitVec<T, C, O> {
    fn new() -> Self {
        Self {
            inner: BitArray::new([0.into(); C]),
            len: 0,
        }
    }

    fn push_front(&mut self, data: &BitSlice<u8, O>) {
        self.inner.shift_right(8);
        self.inner[..8]
            .iter_mut()
            .zip(data)
            .for_each(|(mut l, r)| *l = *r);

        self.len += 8;
    }

    fn push_back(&mut self, data: &BitSlice<u8, O>) {
        self.inner[self.len..]
            .iter_mut()
            .zip(data)
            .for_each(|(mut l, r)| *l = *r);

        self.len += data.len();
    }

    fn pop_front(&mut self, width: usize) -> u8 {
        debug_assert!(width <= 8);
        let mut result = BitArray::<u8, O>::new(0);

        self.inner[..width]
            .iter()
            .zip(result.iter_mut())
            .for_each(|(l, mut r)| *r = *l);

        self.inner.shift_left(width);
        self.len -= width;
        result.data
    }

    fn pop_back(&mut self, width: usize) -> u8 {
        debug_assert!(width <= 8);

        let mut result = BitArray::<u8, O>::new(0);
        self.inner[self.len - width..self.len]
            .iter()
            .zip(result.iter_mut())
            .for_each(|(l, mut r)| *r = *l);

        self.len -= width;
        result.data
    }

    fn as_bitslice(&self) -> &BitSlice<T, O> {
        &self.inner[..self.len]
    }
}

impl<T: BitStore, const C: usize, O: BitOrder> Debug for BoundedBitVec<T, C, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", &self.inner[..self.len])
    }
}
