use crate::{
    transport::{Command, Decoder},
    Device, Encoder, Frame, IndexMap, FRAME_DATA_LEN,
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

pub struct Connection<D: Device> {
    is_controller: bool,
    device: D,
    counter: u32,
    state: State,
    encoder: Encoder,
    decoder: Decoder,
    to_be_sent: Option<[u8; FRAME_DATA_LEN]>,
    just_received: Option<[u8; FRAME_DATA_LEN]>,
    sent: IndexMap<Frame>,
    received: IndexMap<Frame>,
}

impl<D: Device> Connection<D> {
    pub fn new(device: D, is_controller: bool) -> Self {
        Self {
            is_controller,
            device,
            counter: 0,
            state: State::WaitingForConnection,
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            to_be_sent: None,
            just_received: None,
            sent: IndexMap::new(Frame::empty_invalid()),
            received: IndexMap::new(Frame::empty_invalid()),
        }
    }

    pub fn send(&mut self, data: [u8; FRAME_DATA_LEN]) -> bool {
        if self.to_be_sent.is_none() {
            self.to_be_sent = Some(data);
        }
        self.to_be_sent.is_some()
    }

    pub fn receive(&mut self) -> Option<[u8; FRAME_DATA_LEN]> {
        self.just_received.take()
    }

    pub fn poll(&mut self) {
        if self.counter % 32 == 0 && self.state == State::WaitingForConnection {
            self.encoder.send_sync();
        }

        if self.counter % 4 == 0 {
            // if self.state == State::OnlySending || self.state == State::SendingAndReceiving {
            //     if let Some(data) = self.to_be_sent {
            //         if let Some(frame) = self.sent.try_insert(|index| Frame::new(index, data)) {
            //             self.encoder.send_frame(frame);
            //             self.to_be_sent = None;
            //         }
            //     }
            // }

            self.encoder.poll(&mut self.device);
        }

        let command = self.decoder.poll(&mut self.device);

        #[cfg(target_arch = "avr")]
        ufmt::uwriteln!(self.device.serial(), "{:?}\r", command).unwrap();
        #[cfg(not(target_arch = "avr"))]
        println!("{:?}", command);

        match command {
            Command::Sync => {
                // #[cfg(target_arch = "avr")]
                // ufmt::uwriteln!(self.device.serial(), "Sync requested").unwrap();

                if self.is_controller {
                    self.state_transition(State::SendingAndReceiving);
                } else {
                    self.encoder.send_sync();
                    self.state_transition(State::SendingAndReceiving);
                }
            }

            Command::Frame(frame) if frame.is_valid() => {
                // self.received
                //     .insert_at(frame.index, frame)
                //     .expect("Entry already taken");
                // self.encoder.send_ack(frame.index);
            }
            Command::Frame(frame) => {
                // self.encoder.send_nack(frame.index);
            }

            Command::Ack(index) => {
                // self.received.remove(index)
            }
            Command::Nack(index) => {
                // self.encoder.send_frame(self.received.get(index).unwrap())
            }

            Command::Finished => self.state_transition(State::OnlySending),
            Command::None => (),
        }

        self.counter = self.counter.wrapping_add(1);
    }

    fn state_transition(&mut self, next: State) {
        use State::*;
        dbg!((self.state, next));

        #[cfg(target_arch = "avr")]
        let mut log = |str| ufmt::uwrite!(self.device.serial(), "{}\r", str).unwrap();

        #[cfg(not(target_arch = "avr"))]
        let log = |str: &'static str| {
            println!("{}", str);
        };

        match (self.state, next) {
            (WaitingForConnection, SendingAndReceiving) => log("== Established connection =="),
            (SendingAndReceiving, SendingAndReceiving) => log("== Reestablished connection =="),
            (SendingAndReceiving, OnlySending) => log("== Done receiving =="),
            (SendingAndReceiving, OnlyReceiving) => log("== Done sending =="),
            _ => unreachable!(),
        }

        self.state = next;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    WaitingForConnection,
    SendingAndReceiving,
    OnlySending,
    OnlyReceiving,
}
