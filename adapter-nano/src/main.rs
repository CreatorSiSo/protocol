#![no_std]
#![no_main]

mod panic_handler;

use arduino_hal::{
    hal::port::{PB0, PB1, PD0, PD1, PD2, PD3, PD4, PD5, PD6, PD7},
    pac::USART0,
    port::{
        mode::{Floating, Input, Output},
        Pin,
    },
    prelude::_unwrap_infallible_UnwrapInfallible,
    Usart,
};
use common::{self, BitIter, BitVec, Connection, Device, MirrorConnection};

struct ArduinoDevice {
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>,
    d2: Pin<Output, PD2>,
    d3: Pin<Output, PD3>,
    d4: Pin<Output, PD4>,
    d5: Pin<Output, PD5>,
    d6: Pin<Input<Floating>, PD6>,
    d7: Pin<Input<Floating>, PD7>,
    d8: Pin<Input<Floating>, PB0>,
    d9: Pin<Input<Floating>, PB1>,
}

impl ArduinoDevice {
    fn get(&mut self, pin: u8) -> bool {
        match pin {
            6 => self.d6.is_high(),
            7 => self.d7.is_high(),
            8 => self.d8.is_high(),
            9 => self.d9.is_high(),
            _ => panic!("Cannot read pin {pin}!"),
        }
    }

    fn set(&mut self, pin: u8, value: bool) {
        match pin {
            2 if value => self.d2.set_high(),
            2 if !value => self.d2.set_low(),
            3 if value => self.d3.set_high(),
            3 if !value => self.d3.set_low(),
            4 if value => self.d4.set_high(),
            4 if !value => self.d4.set_low(),
            5 if value => self.d5.set_high(),
            5 if !value => self.d5.set_low(),
            _ => panic!("Cannot write to pin {pin}!"),
        }
    }
}

impl Device for ArduinoDevice {
    fn read(&mut self) -> BitVec<1> {
        let mut result = BitVec::new();
        for pin in 6..=9 {
            result.push_back(&self.get(pin));
        }
        for bit in result.iter() {
            ufmt::uwrite!(self.serial, "{}, ", if bit { 1 } else { 0 }).unwrap_infallible();
        }
        ufmt::uwriteln!(self.serial, "\r").unwrap_infallible();
        result
    }

    fn write(&mut self, data: BitVec<1>) {
        for (value, pin) in data.iter().zip(2..=5) {
            self.set(pin, value);
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let device = ArduinoDevice {
        serial: arduino_hal::default_serial!(dp, pins, 57600),
        d2: pins.d2.into_output(),
        d3: pins.d3.into_output(),
        d4: pins.d4.into_output(),
        d5: pins.d5.into_output(),
        d6: pins.d6,
        d7: pins.d7,
        d8: pins.d8,
        d9: pins.d9,
    };

    let mut connection = MirrorConnection::new(device);

    loop {
        connection.poll();
        arduino_hal::delay_ms(100);
    }
}
