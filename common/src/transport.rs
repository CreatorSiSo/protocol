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
const BIT_WIDTH: u8 = 3;

pub struct TransportEncode<D: Device> {
    device: D,
    // buffer of variable size with data to be sent
    data: [u8; 64],
    // length of the data to be sent in bits
    bits: usize,
    clock: u8,
}

impl<D: Device> TransportEncode<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            data: [0; 64],
            bits: 0,
            clock: 0,
        }
    }

    pub fn poll(&mut self) {
        let nibble = self.clock | self.pop(BIT_WIDTH);
        self.device.write(nibble);
        self.clock = if self.clock == 0 { 0b00001000 } else { 0 }
    }

    pub fn push(&mut self, byte: u8) {
        let u8_bits = 8;

        let left_index = self.data.len() - (self.bits / u8_bits) - 1;
        let right_index = left_index - 1;
        dbg!(right_index);
        dbg!(left_index);

        let shift_by = (self.bits % u8_bits) as u8;
        dbg!(shift_by);

        let right_byte = byte << shift_by;
        let left_byte = byte.rotate_left(u8::BITS - shift_by as u32) & bitmask_lower(shift_by);
        dbg!(right_byte);
        dbg!(left_byte);

        self.data[right_index] = right_byte;
        self.data[left_index] = left_byte;

        self.bits += u8_bits;
    }

    fn pop(&mut self, width: u8) -> u8 {
        debug_assert!(width <= 8);
        // get the last `width` bits of the last byte
        let popped = self.data[self.data.len() - 1] & dbg!(bitmask_lower(width));
        slice_bit_shift_right(&mut self.data, width as u32);
        self.bits -= width as usize;
        popped
    }
}

const fn bitmask_lower(width: u8) -> u8 {
    (1 << width) - 1
}

#[test]
fn encode() {
    todo!()
    // let data = [0xf0; 4];
    // let mut encoder = TransportEncode::<B15fStud>::new();
    // encoder.push(0xff);
    // encoder.poll();
    // encoder.poll();
    // for byte in data {
    //     encoder.push(byte);
    // }
    // assert_eq!(
    //     &[0b00000011, 0b11000011, 0b11000011, 0b11000011, 0b11000011],
    //     encoder.data.last_chunk::<5>().unwrap()
    // );
}

fn slice_bit_shift_left<const N: usize>(slice: &mut [u8; N], by: u32) {
    let mut prev = [0; N];
    prev.copy_from_slice(slice);

    for (index, byte) in slice.iter_mut().enumerate() {
        *byte <<= by;
        *byte |= ((index + 1) < N)
            .then_some(index + 1)
            .map(|index_to_right| prev[index_to_right] >> (u8::BITS - by))
            .unwrap_or(0x00);
    }
}

fn slice_bit_shift_right<const N: usize>(slice: &mut [u8; N], by: u32) {
    let mut prev = [0; N];
    prev.copy_from_slice(slice);

    for (index, byte) in slice.iter_mut().enumerate() {
        *byte >>= by;
        *byte |= index
            .checked_sub(1)
            .map(|index_to_left| prev[index_to_left] << by)
            .unwrap_or(0x00);
    }
}

#[test]
fn bit_shift_slices() {
    let mut array = [0b11111111, 0b11111111, 0, 0, 0, 0, 0, 0];
    slice_bit_shift_left(&mut array, 3);
    assert_eq!(array, [0b11111111, 0b11111000, 0, 0, 0, 0, 0, 0]);

    array = [0, 0, 0, 0, 0, 0, 0b11111111, 0b11111111];
    slice_bit_shift_right(&mut array, 3);
    assert_eq!(array, [0, 0, 0, 0, 0, 0, 0b00011111, 0b11111111]);
}
