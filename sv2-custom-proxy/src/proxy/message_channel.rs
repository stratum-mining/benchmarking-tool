use crate::proxy::into_static;
use crate::proxy::{Frame_, StdFrame};
use async_channel::{Receiver, Sender};
use codec_sv2::framing_sv2::framing::Frame as EitherFrame;
use stratum_common::roles_logic_sv2::parsers_sv2::AnyMessage;
use tracing::{debug, error, trace};

pub type MessageType = u8;

#[derive(PartialEq, Debug)]
pub enum Remote {
    Client,
    Server,
}

pub struct MessageChannel {
    pub message_type: MessageType,
    pub expect_from: Remote,
    pub receiver: Option<Receiver<AnyMessage<'static>>>,
    pub sender: Sender<AnyMessage<'static>>,
}

impl MessageChannel {
    pub async fn on_message(&mut self, frame: &mut Frame_) -> Option<Frame_> {
        debug!("Handler checking message from {:?}", self.expect_from);
        let (mt, message) = self.message_from_frame(frame);
        debug!("Parsed message type {} from {:?}, handler expects type {}", mt, self.expect_from, self.message_type);
        if mt == self.message_type {
            if self.sender.send(message).await.is_err() {
                eprintln!("Impossible to send message to message handler, for: {mt}");
                std::process::exit(1);
            };
            if let Some(receiver) = &mut self.receiver {
                if let Ok(message) = receiver.recv().await {
                    let frame: StdFrame = message
                        .try_into()
                        .expect("A message can always be converted in a frame");
                    Some(frame.into())
                } else {
                    eprintln!("Impossible to receive message from message handler, for: {mt}");
                    std::process::exit(1);
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn message_from_frame(&self, frame: &mut Frame_) -> (u8, AnyMessage<'static>) {
        let expect_from = &self.expect_from;
        trace!("message_from_frame called for frame from {:?}", expect_from);
        match frame {
            EitherFrame::Sv2(frame) => {
                if let Some(header) = frame.get_header() {
                    let mt = header.msg_type();
                    trace!("Frame has message type: {}, getting payload...", mt);
                    let mut payload = frame.payload().to_vec();
                    trace!("Payload length: {} bytes, attempting to parse as AnyMessage", payload.len());
                    let maybe_message: Result<AnyMessage<'_>, _> =
                        (mt, payload.as_mut_slice()).try_into();

                    match maybe_message {
                        Ok(message) => {
                            trace!("Successfully parsed message type {}", mt);
                            (mt, into_static(message))
                        },
                        Err(e) => {
                            error!("Failed to parse message type {} (0x{:02x}): {:?}, from: {}", mt, mt, e, expect_from);
                            eprintln!("Received frame with invalid payload or message type: {frame:?}, from: {expect_from}, error: {e:?}");
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Received frame with invalid header: {frame:?}, from: {expect_from}");
                    std::process::exit(1);
                }
            }
            EitherFrame::HandShake(f) => {
                eprintln!("Received unexpected handshake frame: {f:?}, from: {expect_from}");
                std::process::exit(1);
            }
        }
    }
}

impl std::fmt::Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client => write!(f, "client"),
            Self::Server => write!(f, "server"),
        }
    }
}
