use b15f::B15fDriver;

pub trait Device {
    /// Only sends lower nibble of byte.
    fn send(&mut self, data: u8);

    /// Only reads lower nibble of byte.
    fn read(&self) -> u8;
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


    fn read(&self) -> u8 {
        self.driver.get_register_pina()
    }
}

pub struct Arduino;