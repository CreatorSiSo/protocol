use std::{
    fmt::Debug,
    io::{stdin, Read},
    ops::BitXor,
};

mod types;
use types::*;

fn main() {
    let mut state = State::new();

    let input = stdin().lock().bytes();
    for maybe_byte in input {
        let byte = Byte::from_u8(maybe_byte.unwrap());
        println!("\n{0}", byte);

        state.send(byte.upper_nibble());
        state.send(byte.lower_nibble());
    }
}

struct State {
    clock: Byte,
    output: Nibble,
}

impl State {
    fn new() -> Self {
        Self {
            clock: Byte::from_u8(0),
            output: Nibble::from_u8(0),
        }
    }

    fn send(&mut self, data: Nibble) {
        self.output = data.bitxor(self.clock.lower_nibble());
        self.clock = !self.clock;
        println!("{:?}", &self);
        self.output = data.bitxor(self.clock.lower_nibble());
        self.clock = !self.clock;
        println!("{:?}", &self);
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> ", self.clock.lower_nibble())?;
        write!(f, "{}", self.output)?;
        Ok(())
    }
}
