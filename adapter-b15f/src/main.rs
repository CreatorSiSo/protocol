use b15f::{B15f, B15fDriver};
use common::{BitVec, Connection, Device, Frame, FRAME_DATA_LEN};
use std::{
    io::{stdin, stdout, Read, Write},
    thread,
    time::{Duration, Instant},
};

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
        BitVec::from_bytes([self.driver.get_register_pina().reverse_bits()], 4)
    }

    fn write(&mut self, data: BitVec<1>) {
        self.driver.set_register_porta(data.bytes()[0]);
    }
}

fn main() -> Result<(), &'static str> {
    let mut stdin = stdin().lock();
    let mut stdout = stdout().lock();

    let device = B15fDevice::new()?;
    let mut connection = Connection::new(device);
    let mut input = vec![];
    stdin.read_to_end(&mut input).unwrap();
    let mut chunks = input.chunks(FRAME_DATA_LEN);

    loop {
        let now = Instant::now();
        if connection.poll() {
            return Ok(());
        }

        if let Some(frame) = connection.receive() {
            stdout.write_all(&frame.data[..frame.len as usize]).unwrap();
            stdout.flush().unwrap();
        }
        connection.send(|| {
            chunks.next().map(|chunk| {
                let mut data = [0; FRAME_DATA_LEN];
                data[..chunk.len()].copy_from_slice(chunk);
                Frame::new(chunk.len().try_into().unwrap(), data)
            })
        });

        thread::sleep(Duration::from_millis(30).saturating_sub(now.elapsed()));
        // eprintln!("Actual loop time: {}ms", now.elapsed().as_millis());
    }
}
