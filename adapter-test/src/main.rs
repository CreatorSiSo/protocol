use std::{
    sync::atomic::{AtomicU8, Ordering},
    time::Duration,
};

use common::{BitVec, Connection, Device};

pub struct TestDevice<'a> {
    input: &'a AtomicU8,
    output: &'a AtomicU8,
}

impl Device for TestDevice<'_> {
    fn read(&mut self) -> BitVec<1> {
        let value = self.input.load(Ordering::Relaxed) << 4;
        BitVec::from_bytes([value], 4)
    }

    fn write(&mut self, data: BitVec<1>) {
        let value = data.bytes()[0] >> 4;
        self.output.store(value, Ordering::Relaxed);
    }
}

fn main() {
    let a = AtomicU8::new(0);
    let b = AtomicU8::new(0);
    let mut connection_a = Connection::new(TestDevice {
        input: &a,
        output: &b,
    });
    let mut connection_b = Connection::new(TestDevice {
        input: &b,
        output: &a,
    });

    loop {
        eprintln!(
            "{:04b} {:04b}",
            a.load(Ordering::Relaxed),
            b.load(Ordering::Relaxed)
        );
        connection_a.poll();
        std::thread::sleep(Duration::from_millis(50));
        eprintln!(
            "{:04b} {:04b}",
            a.load(Ordering::Relaxed),
            b.load(Ordering::Relaxed)
        );
        connection_b.poll();
        std::thread::sleep(Duration::from_millis(50));
    }
}
