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
    device: D,
    counter: u32,
    sync_requests: u16,
    state: State,
    encoder: Encoder,
    decoder: Decoder,
    to_be_sent: Option<[u8; FRAME_DATA_LEN]>,
    just_received: Option<[u8; FRAME_DATA_LEN]>,
    sent: IndexMap<Frame>,
    received: IndexMap<Frame>,
}

impl<D: Device> Connection<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            counter: 0,
            sync_requests: 0,
            state: State::WaitingForConnection,
            encoder: Encoder::new(),
            decoder: Decoder::new(),
            to_be_sent: None,
            just_received: None,
            sent: IndexMap::new(Frame::empty_invalid()),
            received: IndexMap::new(Frame::empty_invalid()),
        }
    }

    pub fn send(&mut self, mut f: impl FnMut() -> [u8; FRAME_DATA_LEN]) {
        if self.to_be_sent.is_none() {
            self.to_be_sent = Some(f());
        }
    }

    pub fn receive(&mut self) -> Option<[u8; FRAME_DATA_LEN]> {
        self.just_received.take()
    }

    pub fn poll(&mut self) {
        if self.counter % 16 == 0 && self.state == State::WaitingForConnection {
            if self.sync_requests >= 10 {
                self.encoder.send_sync_res();
            } else {
                self.encoder.send_sync_req()
            }
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

            if self.encoder.needs_data() {
                self.encoder.send_noop();
            }
            self.encoder.poll(&mut self.device);
        }

        let command = self.decoder.poll(&mut self.device);

        if command != Command::None {
            #[cfg(target_arch = "avr")]
            ufmt::uwriteln!(self.device.serial(), "{:?}\r", command).unwrap();
            #[cfg(not(target_arch = "avr"))]
            println!("{:?}", command);
        }

        match command {
            Command::SyncReq => {
                // #[cfg(target_arch = "avr")]
                // ufmt::uwriteln!(self.device.serial(), "Sync requested").unwrap();
                self.sync_requests += 1;
            }
            Command::SyncRes => {
                self.state_transition(State::SendingAndReceiving);
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
        #[cfg(not(target_arch = "avr"))]
        eprintln!("{:?}", (self.state, next));

        #[cfg(target_arch = "avr")]
        let mut log = |str| ufmt::uwrite!(self.device.serial(), "{}\r", str).unwrap();

        #[cfg(not(target_arch = "avr"))]
        let log = |str: &'static str| {
            println!("{}", str);
        };

        match (self.state, next) {
            (WaitingForConnection, SendingAndReceiving) => log("=> Established connection"),
            (SendingAndReceiving, SendingAndReceiving) => log("=> Reestablished connection"),
            (SendingAndReceiving, OnlySending) => log("=> Done receiving"),
            (SendingAndReceiving, OnlyReceiving) => log("=> Done sending"),
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
