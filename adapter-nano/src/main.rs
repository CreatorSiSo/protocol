#![no_std]
#![no_main]

mod panic_handler;

use arduino_hal::{
    hal::port::{PB0, PB1, PD2, PD3, PD4, PD5, PD6, PD7},
    port::{
        mode::{Floating, Input, Output},
        Pin,
    },
};
use common::{self, BitVec, Device, MirrorConnection};

struct ArduinoDevice {
    d2: Pin<Input<Floating>, PD2>,
    d3: Pin<Input<Floating>, PD3>,
    d4: Pin<Input<Floating>, PD4>,
    d5: Pin<Input<Floating>, PD5>,
    d6: Pin<Output, PD6>,
    d7: Pin<Output, PD7>,
    d8: Pin<Output, PB0>,
    d9: Pin<Output, PB1>,
}

impl ArduinoDevice {
    fn get(&mut self, pin: u8) -> bool {
        match pin {
            2 => self.d2.is_high(),
            3 => self.d3.is_high(),
            4 => self.d4.is_high(),
            5 => self.d5.is_high(),
            _ => panic!("Cannot read pin {pin}!"),
        }
    }

    fn set(&mut self, pin: u8, value: bool) {
        match pin {
            6 if value => self.d6.set_high(),
            6 if !value => self.d6.set_low(),
            7 if value => self.d7.set_high(),
            7 if !value => self.d7.set_low(),
            8 if value => self.d8.set_high(),
            8 if !value => self.d8.set_low(),
            9 if value => self.d9.set_high(),
            9 if !value => self.d9.set_low(),
            _ => panic!("Cannot write to pin {pin}!"),
        }
    }
}

impl Device for ArduinoDevice {
    fn read(&mut self) -> BitVec<1> {
        let mut result = BitVec::new();
        for pin in 2..=5 {
            result.push_back_bool(self.get(pin));
        }
        result
    }

    fn write(&mut self, data: BitVec<1>) {
        for (value, pin) in data.iter().zip(6..=9) {
            self.set(pin, value);
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let device = ArduinoDevice {
        d2: pins.d2,
        d3: pins.d3,
        d4: pins.d4,
        d5: pins.d5,
        d6: pins.d6.into_output(),
        d7: pins.d7.into_output(),
        d8: pins.d8.into_output(),
        d9: pins.d9.into_output(),
    };

    let mut connection = MirrorConnection::new(device);

    loop {
        connection.poll();
        arduino_hal::delay_ms(10);
    }
}
