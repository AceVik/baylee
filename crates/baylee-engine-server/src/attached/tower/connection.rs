use super::{Control, Envelope, Failure, Msg};
use futures_util::StreamExt as _;
use prost::Message as _;
use std::collections::VecDeque;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

/// Bounded in-flight human transport while durability or recovery is pending.
/// These are unaccepted inputs, never evidence and never a reason to advance rules.
pub(super) struct Connection<'a, S> {
    socket: &'a mut WebSocketStream<S>,
    waiting: VecDeque<Envelope>,
    bytes: usize,
}

impl<'a, S: AsyncRead + AsyncWrite + Unpin> Connection<'a, S> {
    pub(super) fn new(socket: &'a mut WebSocketStream<S>) -> Self {
        Self {
            socket,
            waiting: VecDeque::new(),
            bytes: 0,
        }
    }

    pub(super) async fn send(&mut self, control: &Control, data: Vec<u8>) -> Result<(), Failure> {
        super::super::send(self.socket, &control.envelope(data)?).await
    }

    pub(super) async fn frames(&mut self, frames: &[Envelope]) -> Result<(), Failure> {
        super::super::send_batch(self.socket, frames).await
    }

    async fn read(&mut self) -> Result<Envelope, Failure> {
        loop {
            let frame = self
                .socket
                .next()
                .await
                .ok_or("Tower attachment closed")??;
            if frame.is_close() {
                return Err("Tower attachment closed".into());
            }
            if !frame.is_binary() {
                continue;
            }
            let envelope = Envelope::decode(frame.into_data())?;
            if matches!(&envelope.msg, Some(Msg::Error(_))) {
                return Err("Tower host refused execution".into());
            }
            return Ok(envelope);
        }
    }

    pub(super) async fn next(&mut self) -> Result<Envelope, Failure> {
        if let Some(envelope) = self.waiting.pop_front() {
            self.bytes -= envelope.encoded_len();
            return Ok(envelope);
        }
        let envelope = self.read().await?;
        human(&envelope)?;
        Ok(envelope)
    }

    pub(super) async fn control(&mut self) -> Result<(Control, Vec<u8>), Failure> {
        loop {
            let envelope = self.read().await?;
            if let Some(Msg::TowerFrame(frame)) = envelope.msg {
                return Ok(Control::decode(frame)?);
            }
            human(&envelope)?;
            self.bytes = self
                .bytes
                .checked_add(envelope.encoded_len())
                .filter(|n| *n <= 4 * 1024 * 1024)
                .ok_or("Tower pending input capacity")?;
            if self.waiting.len() >= 64 {
                return Err("Tower pending input capacity".into());
            }
            self.waiting.push_back(envelope);
        }
    }
}

fn human(envelope: &Envelope) -> Result<(), Failure> {
    match &envelope.msg {
        Some(Msg::SeatAttached(value)) if value.seat == 0 => Ok(()),
        Some(Msg::SeatDetached(value)) if value.seat == 0 => Ok(()),
        Some(Msg::SeatFrame(value)) if value.seat == 0 => Ok(()),
        Some(Msg::CloseGame(_)) => Ok(()),
        _ => Err("Tower frame direction refused".into()),
    }
}
