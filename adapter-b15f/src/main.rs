use b15f::{B15f, B15fDriver};
use common::Device;
use std::io::{Read, stdin, stdout};
use std::{thread, time::Duration};

fn main() -> Result<(), &'static str> {
    let stdin = stdin().lock().bytes();
    let stdout = stdout().lock();
    // let input = fs::read("./data/random-256.bin").unwrap();

    // let mut connection = Connection::new(stdin, std::iter::empty(), stdout);

    // while connection.poll() {
    //     thread::sleep(Duration::from_millis(1));
    // }

    Ok(())
}

pub struct B15fDevice {
    driver: B15fDriver,
}

impl B15fDevice {
    pub fn new() -> Result<Self, &'static str> {
        let mut driver = B15fDriver::new()?;
        // Use lower nibble of the register to send data
        driver.set_register_ddra(0x0f);
        Ok(Self { driver })
    }
}

impl Device for B15fDevice {
    const NAME: &'static str = "B15f";

    fn read(&mut self) -> u8 {
        self.driver.get_register_pina()
    }

    fn write(&mut self, data: u8) {
        self.driver.set_register_porta(data);
    }
}
