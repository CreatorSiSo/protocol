use crate::{
    transport::{Command, Decoder},
    Device, Encoder, Frame, IndexMap,
};

#[cfg(target_arch = "avr")]
macro_rules! dbg {
    () => {};
    ($val:expr $(,)?) => {
        match $val {
            val => val,
        }
    };
    ($($val:expr),+ $(,)?) => {
        $val
    };
}

#[cfg(target_arch = "avr")]
fn log(_str: &'static str) {}

#[cfg(not(target_arch = "avr"))]
fn log(str: &'static str) {
    eprintln!("{}", str);
}

pub struct MirrorConnection<D: Device> {
    counter: u32,
    device: D,
    encoder: Encoder,
    decoder: Decoder,
    state: State,
    received: IndexMap<Frame>,
}

impl<D: Device> MirrorConnection<D> {
    pub fn new(device: D) -> Self {
        Self {
            counter: 0,
            device,
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            state: State::WaitingForConnection,
            received: IndexMap::new(Frame::empty_invalid()),
        }
    }
}

impl<D: Device> Connection for MirrorConnection<D> {
    fn poll(&mut self) {
        dbg!(&self.received);

        if self.counter % 4 == 0 {
            if self.state == State::WaitingForConnection {
                self.encoder.send_sync();
            }

            if self.state == State::OnlySending || self.state == State::SendingAndReceiving {
                if let Some(mut entry) = self.received.oldest() {
                    self.encoder.send_frame(entry.get());
                    // TODO Only remove if encoder is not full
                    entry.remove();
                }
            }

            self.encoder.poll(&mut self.device);
        }

        match self.decoder.poll(&mut self.device) {
            Command::Sync => self.state_transition(State::SendingAndReceiving),

            Command::Frame(frame) if frame.is_valid() => {
                // TODO Figure out what to do if IndexMap is full
                self.received
                    .try_insert(|_| frame)
                    .expect("received IndexMap is full");
                self.encoder.send_ack(frame.index);
            }
            Command::Frame(frame) => {
                self.encoder.send_nack(frame.index);
            }

            Command::Ack(index) => self.received.remove(index),
            Command::Nack(index) => self.encoder.send_frame(self.received.get(index).unwrap()),

            Command::Finished => self.state_transition(State::OnlySending),
            Command::None => (),
        }

        self.counter = self.counter.wrapping_add(1);
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

pub trait Connection {
    fn poll(&mut self);
    fn state_mut(&mut self) -> &mut State;

    fn state_transition(&mut self, next: State) {
        use State::*;
        let state = self.state_mut();

        dbg!((&state, &next));

        match (&state, &next) {
            (WaitingForConnection, SendingAndReceiving) => log("== Established connection =="),
            (SendingAndReceiving, SendingAndReceiving) => log("== Reestablished connection =="),
            (SendingAndReceiving, OnlySending) => log("== Done receiving =="),
            (SendingAndReceiving, OnlyReceiving) => log("== Done sending =="),
            _ => unreachable!(),
        }

        *state = next;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    WaitingForConnection,
    SendingAndReceiving,
    OnlySending,
    OnlyReceiving,
}
