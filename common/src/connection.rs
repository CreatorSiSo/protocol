use crate::{
    transport::{Command, Decoder},
    Device, Encoder, Frame, FrameData,
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
    to_be_sent: FrameData,
    just_received: FrameData,
    sent: Frame,
    received: Frame,
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
            to_be_sent: FrameData::None,
            just_received: FrameData::None,
            sent: Frame::empty_invalid(),
            received: Frame::empty_invalid(),
        }
    }

    pub fn send(&mut self, mut f: impl FnMut() -> FrameData) {
        if self.to_be_sent == FrameData::None {
            self.to_be_sent = f();
        }
    }

    pub fn receive(&mut self) -> FrameData {
        self.just_received.take()
    }

    pub fn poll(&mut self) {
        match self.state {
            State::WaitingForConnection => self.waiting_for_connection(),
            State::SendingAndReceiving => self.sending(),
            State::OnlySending => self.sending(),
            State::OnlyReceiving => self.sending(),
        }

        if self.counter % 4 == 0 {
            if self.encoder.needs_data() {
                self.encoder.send_noop();
            }
            self.encoder.poll(&mut self.device);
        }

        let command = self.decoder.poll(&mut self.device);

        // #[cfg(target_arch = "avr")]
        // ufmt::uwriteln!(self.device.serial(), "cmd {:?}\r", command).unwrap();
        // #[cfg(not(target_arch = "avr"))]
        // eprintln!("cmd {:?}", command);

        match command {
            Command::SyncReq => {
                self.sync_requests += 1;
            }
            Command::SyncRes => {
                self.state_transition(State::SendingAndReceiving);
            }

            Command::Frame(frame) if frame.is_valid() => {
                self.received = frame;
                self.encoder.send_ack(frame.index);
            }
            Command::Frame(frame) => {
                // self.encoder.send_nack(frame.index);
            }

            Command::Ack => {
                // self.received.remove(index)
            }
            Command::Nack => {
                // self.encoder.send_frame(self.received.get(index).unwrap())
            }

            Command::Finished => self.state_transition(State::OnlySending),
            Command::None => (),
        }

        self.counter = self.counter.wrapping_add(1);
    }

    fn waiting_for_connection(&mut self) {
        if self.counter % 16 == 0 {
            if self.sync_requests >= 10 {
                self.encoder.send_sync_res();
            } else {
                self.encoder.send_sync_req()
            }
        }
    }

    fn sending(&mut self) {
        if self.encoder.needs_data() {
            if let FrameData::Full(data) | FrameData::Last(data, _) = self.to_be_sent {
                let frame = Frame::new(0, data);
                self.sent = frame;
                dbg!(frame);
                #[cfg(target_arch = "avr")]
                ufmt::uwriteln!(self.device.serial(), "{:?}\r", frame).unwrap();

                self.encoder.send_frame(&frame);
                self.to_be_sent = FrameData::None;
            }
        }
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
