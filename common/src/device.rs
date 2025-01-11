use crate::bitvec::BitVec;

#[cfg(target_arch = "avr")]
use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};

pub trait Device {
    /// Only reads upper nibble of byte.
    fn read(&mut self) -> BitVec<1>;

    /// Only sends upper nibble of byte.
    fn write(&mut self, data: BitVec<1>);

    #[cfg(target_arch = "avr")]
    fn serial(&mut self) -> &mut Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;
}
