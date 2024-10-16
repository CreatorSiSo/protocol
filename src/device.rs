use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
};

use b15f::B15fDriver;

pub trait Device {
    /// Only sends lower nibble of byte.
    fn send(&mut self, data: u8);

    /// Only reads lower nibble of byte.
    fn read(&mut self) -> u8;
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
    fn send(&mut self, data: u8) {
        self.driver.set_register_porta(data);
    }

    fn read(&mut self) -> u8 {
        self.driver.get_register_pina()
    }
}

pub struct Arduino;

pub struct FileDevice {
    file: File,
}

impl FileDevice {
    pub fn new() -> Self {
        Self {
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open("/tmp/protocol.cable")
                .unwrap(),
        }
    }
}

impl Device for FileDevice {
    fn send(&mut self, data: u8) {
        self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
        self.file.write_all(&[data & 0x0f]).unwrap();
    }

    fn read(&mut self) -> u8 {
        let mut buffer = [0; 1];
        self.file.read(&mut buffer).unwrap();
        buffer[0]
    }
}
