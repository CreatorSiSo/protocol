use std::{io, iter};

use b15f::B15fDriver;

use crate::Connection;

pub trait Device {
    const NAME: &'static str;

    /// Only sends lower nibble of byte.
    fn send(&mut self, data: u8);

    /// Only reads lower nibble of byte.
    fn read(&self) -> u8;

    /// TODO Remove, only used for debugging
    fn debug_poll(&mut self) {}

    fn name(&self) -> &'static str {
        Self::NAME
    }
}

pub struct B15fDevice {
    driver: B15fDriver,
}

impl B15fDevice {
    pub fn new() -> Result<Self, &'static str> {
        let mut driver = B15fDriver::new()?;
        driver.set_register_ddra(0x0f);
        Ok(Self { driver })
    }
}

impl Device for B15fDevice {
    const NAME: &'static str = "B15f";

    fn send(&mut self, data: u8) {
        self.driver.set_register_porta(data);
    }

    fn read(&self) -> u8 {
        self.driver.get_register_pina()
    }
}

pub struct Arduino;

pub struct DebugDevice {
    other_side: Connection<MirrorDevice, iter::Empty<io::Result<u8>>>,
}

impl DebugDevice {
    pub fn new() -> Self {
        Self {
            other_side: Connection::new(MirrorDevice::new(), iter::empty()),
        }
    }
}

impl Device for DebugDevice {
    const NAME: &'static str = "Debug";

    fn send(&mut self, data: u8) {
        eprintln!("{} {:04b}", self.name(), data);
        self.other_side.device.incoming = data;
    }

    fn read(&self) -> u8 {
        self.other_side.device.outgoing
    }

    fn debug_poll(&mut self) {
        self.other_side.poll();
    }
}

pub struct MirrorDevice {
    incoming: u8,
    outgoing: u8,
}

impl MirrorDevice {
    fn new() -> Self {
        Self {
            incoming: 0,
            outgoing: 0,
        }
    }
}

impl Device for MirrorDevice {
    const NAME: &'static str = "Mirror";

    fn send(&mut self, data: u8) {
        eprintln!("{} {:04b}", self.name(), data);
        self.outgoing = data;
    }

    fn read(&self) -> u8 {
        self.incoming
    }
}
