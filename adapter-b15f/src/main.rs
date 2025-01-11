use b15f::{B15f, B15fDriver};
use common::{BitVec, Connection, Device};
use std::{thread, time::Duration};

pub struct B15fDevice {
    driver: B15fDriver,
}

impl B15fDevice {
    pub fn new() -> Result<Self, &'static str> {
        let mut driver = B15fDriver::new()?;
        // Use lower nibble of the register to send data
        driver.set_register_ddra(0xf0);
        Ok(Self { driver })
    }
}

impl Device for B15fDevice {
    fn read(&mut self) -> BitVec<1> {
        BitVec::from_bytes([self.driver.get_register_pina() << 4], 4)
    }

    fn write(&mut self, data: BitVec<1>) {
        self.driver.set_register_porta(data.bytes()[0]);
    }
}

fn main() -> Result<(), &'static str> {
    // let stdin = stdin().lock().bytes();
    // let stdout = stdout().lock();
    // let input = fs::read("./data/random-256.bin").unwrap();

    let device = B15fDevice::new()?;
    let mut connection = Connection::new(device);

    loop {
        // let now = Instant::now();
        connection.poll();
        thread::sleep(Duration::from_millis(20));
        // println!("Actual loop time: {}ms", now.elapsed().as_millis());
    }
}
