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
    finished_counter: u8,
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
            finished_counter: 0,
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

    pub fn poll(&mut self) -> bool {
        match self.state {
            State::WaitingForConnection => self.waiting_for_connection(),
            State::SendingAndReceiving | State::OnlySending => self.sending(),
            State::OnlyReceiving | State::Finished => {}
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
            eprintln!("received {:?}", command);
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
            Command::Finished => {
                if self.state == State::OnlyReceiving {
                    self.state_transition(State::Finished);
                } else {
                    self.state_transition(State::OnlySending);
                }
            }
            Command::None => (),
        }

        self.counter = self.counter.wrapping_add(1);

        if self.state == State::Finished {
            self.finished_counter += 1;
        }
        self.state == State::Finished && self.finished_counter > 10
    }

    fn waiting_for_connection(&mut self) {
        if self.counter % 8 == 0 && self.encoder.needs_data() {
            if self.sync_requests >= 10 {
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
                eprintln!("sending {:?}", frame);
                #[cfg(target_arch = "avr")]
                {
                    ufmt::uwriteln!(self.device.serial(), "sending {:?}\r", frame).unwrap();
                    self.device.serial().flush();
                }

                self.encoder.send_frame(&frame);
            } else {
                self.encoder.send_finished();
                self.state_transition(State::OnlyReceiving);
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

        let do_transition = match (self.state, next) {
            (WaitingForConnection, SendingAndReceiving) => {
                log("=> Established connection");
                true
            }
            (SendingAndReceiving, SendingAndReceiving) => {
                log("=> Reestablished connection");
                true
            }
            (SendingAndReceiving, OnlySending) => {
                log("=> Done receiving");
                true
            }
            (SendingAndReceiving, OnlyReceiving) => {
                log("=> Done sending");
                true
            }
            (_, Finished) => {
                log("=> Finished");
                true
            }
            _ => {
                log("=> Invalid transition");
                false
            }
        };

        if do_transition {
            self.state = next;
        }
    }
}

#[derive(ufmt::derive::uDebug, Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    WaitingForConnection,
    SendingAndReceiving,
    OnlySending,
    OnlyReceiving,
    Finished,
}
