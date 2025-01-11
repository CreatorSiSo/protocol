use b15f::{B15f, B15fDriver};
use common::{BitVec, Connection, Device, FRAME_DATA_LEN, Frame};
use std::{
    io::{ErrorKind, Read, Write, stdin, stdout},
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
    let mut reading = true;
    let mut stdin = stdin().lock();
    let mut stdout = stdout().lock();

    let device = B15fDevice::new()?;
    let mut connection = Connection::new(device);

    loop {
        let now = Instant::now();
        connection.poll();

        if let Some(frame) = connection.receive() {
            stdout.write_all(&frame.data[..frame.len as usize]).unwrap();
        }
        connection.send(|| {
            let mut data = [0; FRAME_DATA_LEN];
            match stdin.read_exact(&mut data).map_err(|err| err.kind()) {
                Result::Ok(..) => Some(Frame::new(FRAME_DATA_LEN as u8, data)),
                Result::Err(ErrorKind::UnexpectedEof) => {
                    if reading {
                        let len = stdin.read(&mut data).unwrap();
                        reading = false;
                        Some(Frame::new(len as u8, data))
                    } else {
                        None
                    }
                }
                Result::Err(kind) => panic!("{}", kind),
            }
        });

        thread::sleep(Duration::from_millis(30).saturating_sub(now.elapsed()));
        // eprintln!("Actual loop time: {}ms", now.elapsed().as_millis());
    }
}
