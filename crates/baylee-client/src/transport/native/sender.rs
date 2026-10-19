//! Bounded, nonblocking outgoing admission, including during a stalled write.

use super::receiver::Handler;
use ewebsock::{WsEvent, WsMessage};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, TryRecvError};

const MAX_MESSAGES: usize = 256;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const OVERFLOW: &str = "WebSocket send queue exceeded its budget";

pub(crate) struct WsSender {
    tx: Option<mpsc::SyncSender<Queued>>,
    bytes: Arc<AtomicUsize>,
    aborted: Arc<AtomicBool>,
    on_event: Handler,
    max_bytes: usize,
}

pub(super) struct SocketQueue {
    rx: mpsc::Receiver<Queued>,
    aborted: Arc<AtomicBool>,
    receiver_gone: Arc<AtomicBool>,
}

pub(super) struct Queued {
    pub(super) message: WsMessage,
    // Kept until this queued message has been written or discarded.
    pub(super) charge: Charge,
}

pub(super) struct Charge {
    bytes: usize,
    total: Arc<AtomicUsize>,
}

impl Drop for Charge {
    fn drop(&mut self) {
        self.total.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

pub(super) fn channel(
    on_event: Handler,
    receiver_gone: Arc<AtomicBool>,
) -> (WsSender, SocketQueue) {
    with_limits(on_event, receiver_gone, MAX_MESSAGES, MAX_BYTES)
}

fn with_limits(
    on_event: Handler,
    receiver_gone: Arc<AtomicBool>,
    max_messages: usize,
    max_bytes: usize,
) -> (WsSender, SocketQueue) {
    let (tx, rx) = mpsc::sync_channel(max_messages);
    let aborted = Arc::new(AtomicBool::new(false));
    let sender = WsSender {
        tx: Some(tx),
        bytes: Arc::new(AtomicUsize::new(0)),
        aborted: Arc::clone(&aborted),
        on_event,
        max_bytes,
    };
    (
        sender,
        SocketQueue {
            rx,
            aborted,
            receiver_gone,
        },
    )
}

impl WsSender {
    /// Admission never waits for the network. Overflow fails the whole link.
    pub(crate) fn send(&mut self, message: WsMessage) {
        let Some(tx) = &self.tx else { return };
        let bytes = match &message {
            WsMessage::Unknown(_) => return,
            WsMessage::Text(text) => text.len(),
            WsMessage::Binary(bytes) | WsMessage::Ping(bytes) | WsMessage::Pong(bytes) => {
                bytes.len()
            }
        };
        if self
            .bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes)
                    .filter(|total| *total <= self.max_bytes)
            })
            .is_err()
        {
            self.abort();
            return;
        }
        let queued = Queued {
            message,
            charge: Charge {
                bytes,
                total: Arc::clone(&self.bytes),
            },
        };
        if let Err(error) = tx.try_send(queued) {
            // Dropping the rejected message releases its byte charge too.
            drop(error);
            self.abort();
        }
    }

    fn abort(&mut self) {
        self.aborted.store(true, Ordering::Release);
        self.tx = None;
        let _ = (self.on_event)(WsEvent::Error(OVERFLOW.into()));
    }
}

impl SocketQueue {
    pub(super) fn try_recv(&self) -> Result<Queued, TryRecvError> {
        if self.aborted.load(Ordering::Acquire) || self.receiver_gone.load(Ordering::Acquire) {
            Err(TryRecvError::Disconnected)
        } else {
            self.rx.try_recv()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::native::receiver::WsReceiver;

    #[test]
    fn stalled_writer_hits_count_and_byte_limits_without_blocking_or_continuing() {
        for (messages, bytes, payload) in [(2, 100, ""), (10, 4, "12")] {
            let (receive, publish) = WsReceiver::new();
            let (mut sender, outgoing) =
                with_limits(publish, receive.cancellation(), messages, bytes);
            for _ in 0..3 {
                sender.send(WsMessage::Text(payload.into()));
            }
            assert!(matches!(receive.try_recv(), Some(WsEvent::Error(error)) if error == OVERFLOW));
            assert!(matches!(
                outgoing.try_recv(),
                Err(TryRecvError::Disconnected)
            ));
            for _ in 0..1000 {
                sender.send(WsMessage::Text("ignored".into()));
            }
            assert!(receive.try_recv().is_none());
            drop(outgoing);
            assert_eq!(sender.bytes.load(Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn normal_send_preserves_order_and_releases_budget_when_written() {
        let (receive, publish) = WsReceiver::new();
        let (mut sender, outgoing) = with_limits(publish, receive.cancellation(), 2, 5);
        sender.send(WsMessage::Text("12".into()));
        sender.send(WsMessage::Binary(vec![3, 4, 5]));
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 5);
        let first = outgoing.try_recv().ok().unwrap();
        assert!(matches!(&first.message, WsMessage::Text(value) if value == "12"));
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 5);
        drop(first);
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 3);
        sender.send(WsMessage::Text("ab".into()));
        let second = outgoing.try_recv().ok().unwrap();
        assert!(matches!(&second.message, WsMessage::Binary(value) if value == &[3, 4, 5]));
        drop(second);
        assert!(
            matches!(outgoing.try_recv().ok().unwrap().message, WsMessage::Text(value) if value == "ab")
        );
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 0);
        drop(sender);
        assert!(matches!(
            outgoing.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }

    #[test]
    fn disappearing_socket_and_single_oversized_message_fail_closed() {
        let (receive, publish) = WsReceiver::new();
        let (mut sender, outgoing) = with_limits(publish, receive.cancellation(), 2, 4);
        drop(outgoing);
        sender.send(WsMessage::Text("abc".into()));
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 0);
        assert!(matches!(receive.try_recv(), Some(WsEvent::Error(_))));
        let (receive, publish) = WsReceiver::new();
        let (mut sender, _outgoing) = with_limits(publish, receive.cancellation(), 2, 4);
        sender.send(WsMessage::Binary(vec![0; 5]));
        assert_eq!(sender.bytes.load(Ordering::Relaxed), 0);
        assert!(matches!(receive.try_recv(), Some(WsEvent::Error(_))));
    }

    #[test]
    fn dropping_the_ui_receiver_closes_an_idle_socket_without_an_incoming_event() {
        let (receive, publish) = WsReceiver::new();
        let (_sender, outgoing) = channel(publish, receive.cancellation());
        assert!(matches!(outgoing.try_recv(), Err(TryRecvError::Empty)));
        drop(receive);
        assert!(matches!(
            outgoing.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }
}
