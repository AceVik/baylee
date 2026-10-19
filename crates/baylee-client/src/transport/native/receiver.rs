//! A socket must not allocate without bound while the UI is busy or paused.

use ewebsock::{WsEvent, WsMessage};
use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

const MAX_EVENTS: usize = 256;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const OVERFLOW: &str = "WebSocket receive queue exceeded its budget";

#[derive(Default)]
struct Queue {
    events: VecDeque<WsEvent>,
    bytes: usize,
    closed: bool,
}

/// Incoming events, bounded independently of the WebSocket frame parser.
pub(crate) struct WsReceiver {
    queue: Arc<Mutex<Queue>>,
    closed: Arc<AtomicBool>,
}

pub(super) type Handler = Arc<dyn Fn(WsEvent) -> ControlFlow<()> + Send + Sync>;

impl WsReceiver {
    pub(super) fn new() -> (Self, Handler) {
        Self::with_limits(MAX_EVENTS, MAX_BYTES)
    }

    fn with_limits(max_events: usize, max_bytes: usize) -> (Self, Handler) {
        let queue = Arc::new(Mutex::new(Queue::default()));
        let closed = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&closed);
        let sender = Arc::clone(&queue);
        let publish = Arc::new(move |event| {
            let mut queue = sender.lock().unwrap_or_else(PoisonError::into_inner);
            if queue.closed {
                return ControlFlow::Break(());
            }
            let bytes = event_bytes(&event);
            if queue.events.len() >= max_events || bytes > max_bytes.saturating_sub(queue.bytes) {
                // No partial continued connection: discard the pending batch,
                // expose one terminal failure, and stop the socket producer.
                queue.events.clear();
                queue.bytes = 0;
                queue.closed = true;
                stopped.store(true, Ordering::Release);
                queue.events.push_back(WsEvent::Error(OVERFLOW.into()));
                return ControlFlow::Break(());
            }
            queue.closed = matches!(event, WsEvent::Closed | WsEvent::Error(_));
            queue.bytes += bytes;
            queue.events.push_back(event);
            if queue.closed {
                stopped.store(true, Ordering::Release);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        (Self { queue, closed }, publish)
    }

    /// Also observed during idle socket loops, not only on the next event.
    pub(super) fn cancellation(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.closed)
    }

    #[cfg(test)]
    pub(super) fn try_recv(&self) -> Option<WsEvent> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let event = queue.events.pop_front()?;
        queue.bytes = queue.bytes.saturating_sub(event_bytes(&event));
        Some(event)
    }

    /// Detach exactly the current bounded batch. A producer cannot refill a
    /// drain loop indefinitely or grow the consumer's temporary vector.
    pub(crate) fn drain(&self) -> Vec<WsEvent> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.bytes = 0;
        queue.events.drain(..).collect()
    }
}

impl Drop for WsReceiver {
    fn drop(&mut self) {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.closed = true;
        self.closed.store(true, Ordering::Release);
        queue.events.clear();
        queue.bytes = 0;
    }
}

fn event_bytes(event: &WsEvent) -> usize {
    match event {
        WsEvent::Message(WsMessage::Text(text) | WsMessage::Unknown(text))
        | WsEvent::Error(text) => text.len(),
        WsEvent::Message(
            WsMessage::Binary(bytes) | WsMessage::Ping(bytes) | WsMessage::Pong(bytes),
        ) => bytes.len(),
        WsEvent::Opened | WsEvent::Closed => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> WsEvent {
        WsEvent::Message(WsMessage::Text(value.into()))
    }

    #[test]
    fn stalled_consumer_gets_one_terminal_error_at_the_event_limit() {
        let (receiver, publish) = WsReceiver::with_limits(2, 100);
        assert!(publish(WsEvent::Opened).is_continue());
        assert!(publish(text("")).is_continue());
        assert!(publish(text("")).is_break());
        for _ in 0..1000 {
            assert!(publish(text("flood")).is_break());
        }
        assert!(matches!(receiver.try_recv(), Some(WsEvent::Error(error)) if error == OVERFLOW));
        assert!(receiver.try_recv().is_none());
    }

    #[test]
    fn byte_budget_counts_all_payload_variants_and_accepts_the_exact_boundary() {
        for payload in [
            WsMessage::Text("12345".into()),
            WsMessage::Unknown("12345".into()),
            WsMessage::Binary(vec![0; 5]),
            WsMessage::Ping(vec![0; 5]),
            WsMessage::Pong(vec![0; 5]),
        ] {
            let (receiver, publish) = WsReceiver::with_limits(10, 5);
            assert!(publish(WsEvent::Message(payload)).is_continue());
            assert!(publish(text("x")).is_break());
            assert!(matches!(receiver.try_recv(), Some(WsEvent::Error(_))));
            assert!(receiver.try_recv().is_none());
        }
        let (receiver, publish) = WsReceiver::with_limits(10, 5);
        assert!(publish(text("123456")).is_break());
        assert!(matches!(receiver.try_recv(), Some(WsEvent::Error(_))));
    }

    #[test]
    fn batches_preserve_order_and_free_capacity_without_refilling_the_old_batch() {
        let (receiver, publish) = WsReceiver::with_limits(3, 5);
        assert!(publish(WsEvent::Opened).is_continue());
        assert!(publish(text("12")).is_continue());
        assert!(publish(text("345")).is_continue());
        let batch = receiver.drain();
        assert!(matches!(&batch[0], WsEvent::Opened));
        assert!(matches!(&batch[1], WsEvent::Message(WsMessage::Text(value)) if value == "12"));
        assert!(matches!(&batch[2], WsEvent::Message(WsMessage::Text(value)) if value == "345"));
        assert!(publish(text("abcde")).is_continue());
        assert_eq!(batch.len(), 3);
        assert_eq!(receiver.drain().len(), 1);
        assert!(receiver.drain().is_empty());
    }

    #[test]
    fn explicit_closure_and_receiver_drop_stop_the_producer() {
        for terminal in [WsEvent::Closed, WsEvent::Error("refused".into())] {
            let (receiver, publish) = WsReceiver::new();
            assert!(publish(WsEvent::Opened).is_continue());
            assert!(publish(terminal).is_break());
            assert!(publish(text("after close")).is_break());
            assert_eq!(receiver.drain().len(), 2);
        }
        let (receiver, publish) = WsReceiver::new();
        drop(receiver);
        assert!(publish(text("receiver gone")).is_break());
    }
}
