//! Local games serialize their rules work on one worker, outside the UI thread.
//! The engine remains synchronous and deterministic; only the host transports
//! actions and results. Idle frames neither wake the worker nor copy the record.

use super::{DuelHost, HostMessage, LinkState, LocalHost, ResumePoint};
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_engine::choice::PlayerAction;
use std::sync::{Mutex, mpsc};

struct Reply {
    messages: Vec<HostMessage>,
    record: Vec<u8>,
    resume: Option<ResumePoint>,
}

pub(super) struct BackgroundHost {
    actions: mpsc::Sender<PlayerAction>,
    replies: Mutex<mpsc::Receiver<Reply>>,
    seat: PlayerId,
    deck: Vec<CardIndex>,
    record: Vec<u8>,
    resume: Option<ResumePoint>,
    outstanding: usize,
    failed: bool,
}

impl BackgroundHost {
    pub(super) fn new(host: LocalHost) -> Self {
        Self::spawn(host)
    }

    fn spawn(mut host: impl DuelHost) -> Self {
        let seat = host.seat();
        let deck = host.own_deck();
        let (actions, input) = mpsc::channel();
        // Backpressure bounds undisplayed snapshots to one; sending never
        // blocks the UI, only this worker. Dropping the table disconnects
        // both channels, releasing even a worker waiting to publish.
        let (output, replies) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("baylee-local-game".into())
            .spawn(move || {
                let mut recorded = 0;
                loop {
                    let mut messages = host.poll();
                    // A session bounds one pump. Reaching its limit is more
                    // work, not a new human question; finish on this worker.
                    while host.busy() {
                        messages.extend(host.poll());
                    }
                    let record = host.local_record().unwrap_or_default();
                    let fresh = record[recorded..].to_vec();
                    recorded = record.len();
                    let reply = Reply {
                        messages,
                        record: fresh,
                        resume: host.resume_point(),
                    };
                    if output.send(reply).is_err() {
                        break;
                    }
                    let Ok(action) = input.recv() else {
                        break;
                    };
                    host.submit(action);
                }
                // LocalHost's Drop flushes and compacts its record on this thread.
            })
            .expect("local game worker can start");
        Self {
            actions,
            replies: Mutex::new(replies),
            seat,
            deck,
            record: Vec::new(),
            resume: None,
            outstanding: 1,
            failed: false,
        }
    }
}

impl DuelHost for BackgroundHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        if self.failed {
            return Vec::new();
        }
        // One completed operation per frame. Never drain a producer that can
        // refill the queue faster than the renderer consumes it.
        match self
            .replies
            .get_mut()
            .expect("exclusive receiver")
            .try_recv()
        {
            Ok(reply) => {
                self.outstanding = self.outstanding.saturating_sub(1);
                self.record.extend(reply.record);
                self.resume = reply.resume;
                reply.messages
            }
            Err(mpsc::TryRecvError::Empty) => Vec::new(),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.failed = true;
                vec![HostMessage::Failed(
                    "The local game worker stopped unexpectedly.".into(),
                )]
            }
        }
    }

    fn submit(&mut self, action: PlayerAction) {
        if !self.failed && self.actions.send(action).is_ok() {
            self.outstanding += 1;
        }
    }

    fn ready(&mut self) {}

    fn seat(&self) -> PlayerId {
        self.seat
    }

    fn own_deck(&self) -> Vec<CardIndex> {
        self.deck.clone()
    }

    fn link(&self) -> LinkState {
        if self.failed {
            LinkState::Down
        } else {
            LinkState::Local
        }
    }

    fn local_record(&self) -> Option<&[u8]> {
        Some(&self.record)
    }

    fn resume_point(&self) -> Option<ResumePoint> {
        // A restart hash must describe the complete persisted operation.
        (self.outstanding == 0)
            .then(|| self.resume.clone())
            .flatten()
    }

    fn busy(&self) -> bool {
        self.outstanding != 0 && !self.failed
    }
}

#[cfg(test)]
mod tests;
