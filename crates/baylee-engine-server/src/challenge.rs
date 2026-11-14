//! Evidence measured at the trusted engine edge, never supplied by a seat.
use super::{EngineRunner, PlayerId, Session, v1};
use sha2::{Digest, Sha256};

#[derive(Default)]
pub(super) struct Evidence {
    digest: Sha256,
    waiting_since: Option<u64>,
    human_ms: u64,
    interrupted: bool,
}

impl Evidence {
    pub(super) fn record(&mut self, bytes: &[u8]) {
        self.digest.update(bytes);
    }

    pub(super) fn interrupt(&mut self) {
        self.interrupted = true;
        self.waiting_since = None;
    }
}

impl EngineRunner {
    pub(super) fn is_challenge(&self) -> bool {
        self.session.as_ref().is_some_and(Session::is_challenge)
    }

    /// Arm human-time measurement after sending a batch, before awaiting input.
    /// `elapsed_ms` comes from the host's monotonic process clock.
    pub fn wait_from(&mut self, elapsed_ms: u64) {
        let waiting = self.is_challenge()
            && !self.ended
            && self.curtain.is_none()
            && self.attached.contains(&0)
            && self.session.as_ref().is_some_and(|session| {
                !session.needs_pump() && session.asked_at(PlayerId::new(0)).is_some()
            });
        if waiting {
            self.challenge.waiting_since.get_or_insert(elapsed_ms);
        } else {
            self.challenge.waiting_since = None;
        }
    }

    /// Stop measurement before processing input or doing engine work.
    /// Server scheduling and network latency while the human is asked count;
    /// loading, AI computation and batch transmission do not.
    pub fn measure_until(&mut self, elapsed_ms: u64) {
        if let Some(since) = self.challenge.waiting_since.take() {
            if let Some(duration) = elapsed_ms.checked_sub(since) {
                self.challenge.human_ms = self.challenge.human_ms.saturating_add(duration);
            } else {
                self.challenge.interrupt();
            }
        }
    }

    pub(super) fn challenge_result(
        &self,
        outcome: v1::challenge_result::Outcome,
    ) -> Option<v1::ChallengeResult> {
        self.is_challenge().then(|| v1::ChallengeResult {
            outcome: outcome.into(),
            finished_at_unix_ms: self.now,
            human_ms: (!self.challenge.interrupted
                && (1..=604_800_000).contains(&self.challenge.human_ms))
            .then_some(self.challenge.human_ms),
            record_sha256: self.challenge.digest.clone().finalize().to_vec(),
        })
    }
}
