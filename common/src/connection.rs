use crate::{
    transport::{Command, Decoder},
    Device, Encoder, Frame,
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
    next: Option<Frame>,
    received: Option<Frame>,
    sent: Option<Frame>,
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
            next: None,
            received: None,
            sent: None,
        }
    }

    pub fn send(&mut self, mut f: impl FnMut() -> Option<Frame>) {
        if self.next.is_none() {
            self.next = f();
        }
    }

    pub fn receive(&mut self) -> Option<Frame> {
        self.received.take()
    }

    pub fn poll(&mut self) {
        match self.state {
            State::WaitingForConnection => self.waiting_for_connection(),
            State::SendingAndReceiving => self.sending(),
            State::OnlySending => self.sending(),
            State::OnlyReceiving => self.sending(),
        }

        if self.counter % 3 == 0 {
            if self.encoder.needs_data() {
                self.encoder.send_noop();
            }
            self.encoder.poll(&mut self.device);
        }

        let command = self.decoder.poll(&mut self.device);

        if command != Command::None {
            #[cfg(target_arch = "avr")]
            {
                ufmt::uwriteln!(self.device.serial(), "received {:?}\r", command).unwrap();
                self.device.serial().flush();
            }
            #[cfg(not(target_arch = "avr"))]
            eprintln!("received {:?}\r", command);
        }

        match command {
            Command::SyncReq => {
                self.sync_requests += 1;
            }
            Command::SyncRes => {
                self.state_transition(State::SendingAndReceiving);
            }

            Command::Frame(frame) => {
                if frame.is_valid() {
                    self.received = Some(frame);
                    self.encoder.send_ack();
                } else {
                    self.encoder.send_nack();
                }
            }

            Command::Ack => {
                self.sent = None;
            }
            Command::Nack => {
                if let Some(frame) = self.received {
                    self.encoder.send_frame(&frame);
                }
            }

            Command::Finished => self.state_transition(State::OnlySending),
            Command::None => (),
        }

        self.counter = self.counter.wrapping_add(1);
    }

    fn waiting_for_connection(&mut self) {
        if self.counter % 8 == 0 {
            if self.sync_requests >= 5 {
                self.encoder.send_sync_res();
            } else {
                self.encoder.send_sync_req()
            }
        }
    }

    fn sending(&mut self) {
        if self.encoder.needs_data() && self.sent.is_none() {
            if let Some(frame) = self.next.take() {
                self.sent = Some(frame);

                #[cfg(not(target_arch = "avr"))]
                eprintln!("sending {:?}\r", frame);
                #[cfg(target_arch = "avr")]
                {
                    ufmt::uwriteln!(self.device.serial(), "sending {:?}\r", frame).unwrap();
                    self.device.serial().flush();
                }

                self.encoder.send_frame(&frame);
            }
        }
    }

    fn state_transition(&mut self, next: State) {
        use State::*;
        #[cfg(not(target_arch = "avr"))]
        eprintln!("{:?}", (self.state, next));
        #[cfg(target_arch = "avr")]
        {
            ufmt::uwrite!(self.device.serial(), "{:?}, {:?}\r", self.state, next).unwrap();
            self.device.serial().flush();
        }

        #[cfg(target_arch = "avr")]
        let mut log = |str| {
            ufmt::uwrite!(self.device.serial(), "{}\r", str).unwrap();
            self.device.serial().flush();
        };
        #[cfg(not(target_arch = "avr"))]
        let log = |str: &'static str| {
            eprintln!("{}", str);
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

#[derive(ufmt::derive::uDebug, Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    WaitingForConnection,
    SendingAndReceiving,
    OnlySending,
    OnlyReceiving,
}
