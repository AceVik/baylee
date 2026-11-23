//! A protected table has exactly one in-flight journal position. The only
//! speculative output allowed out of this process is its private vote.

use crate::variant::HostedSession;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use prost::Message as _;
use sha2::{Digest as _, Sha256};

mod clock;
mod operation;
mod setup;
#[cfg(test)]
mod tests;

type Frames = Vec<(PlayerId, Envelope)>;
type Hash = [u8; 32];

struct Position {
    request: Hash,
    vote: v1::RankedEngineVote,
}

struct Committed {
    position: Position,
    chain: Hash,
}

/// A single execution of the frozen game. Drop it and replay the journal to
/// change generation or abandon a tentative step.
pub struct Replica {
    session: HostedSession,
    game_id: String,
    generation: u64,
    primary: bool,
    committed: Option<Committed>,
    staged: Option<(Position, Frames)>,
}

pub(super) fn uuid_v7(bytes: &[u8]) -> bool {
    uuid::Uuid::from_slice(bytes)
        .is_ok_and(|id| id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
}

fn hash_fields(fields: &[&[u8]]) -> Hash {
    let mut hash = Sha256::new();
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field);
    }
    hash.finalize().into()
}

fn wrap(msg: Msg) -> Envelope {
    Envelope { msg: Some(msg) }
}

impl Replica {
    /// Handles only the private replica protocol. A wrong control sequence
    /// closes this execution without publishing or recording a result.
    ///
    /// # Errors
    /// A stale generation, conflicting retry, gap, or inconsistent commit.
    pub fn handle(&mut self, envelope: Envelope) -> Result<Vec<Envelope>, &'static str> {
        match envelope.msg {
            Some(Msg::RankedEngineStep(step)) => self.step(step),
            Some(Msg::RankedEngineCommit(commit)) => self.commit(&commit),
            Some(Msg::RankedEngineSnapshot(snapshot)) => self.snapshot(&snapshot),
            _ => Err("unexpected protected engine control"),
        }
    }

    fn step(&mut self, step: v1::RankedEngineStep) -> Result<Vec<Envelope>, &'static str> {
        if step.generation != self.generation
            || step.seat > 1
            || !uuid_v7(&step.command_id)
            || step.operation.len() > operation::MAX_BYTES
        {
            return Err("invalid protected step");
        }
        // Generation is a fence for this process. Historical inputs replay on
        // the promoted generation with the same evidence.
        let request = hash_fields(&[
            b"baylee/ranked/operation/v1",
            &step.position.to_be_bytes(),
            &step.command_id,
            &step.seat.to_be_bytes(),
            &[u8::from(step.authority)],
            &step.operation,
        ]);
        let head = self
            .committed
            .as_ref()
            .ok_or("uncommitted protected setup")?;
        for previous in self
            .staged
            .as_ref()
            .map(|(p, _)| p)
            .into_iter()
            .chain([&head.position])
        {
            if previous.vote.position == step.position {
                if previous.request != request {
                    return Err("conflicting protected step");
                }
                return Ok(vec![wrap(Msg::RankedEngineVote(previous.vote.clone()))]);
            }
        }
        if self.staged.is_some()
            || head.position.vote.position.checked_add(1) != Some(step.position)
            || head.position.vote.outcome != 0
        {
            return Err("protected step outside current position");
        }
        let previous = head.position.vote.evidence.clone();
        let seat = PlayerId::new(u8::try_from(step.seat).map_err(|_| "invalid seat")?);
        // The journal carries ordered positions and Central's clock rulings,
        // but no Unix timestamp. Leave the record's time unspecified (zero).
        // A host-local clock would make the independently recorded bytes differ.
        let result = operation::apply(
            &mut self.session,
            &self.game_id,
            seat,
            step.authority,
            &step.operation,
        );
        let accepted = result.is_ok();
        let mut frames = result.unwrap_or_else(|_| {
            let mut frames = vec![(seat, crate::error("ranked command refused"))];
            frames.extend(
                self.session
                    .reask(seat)
                    .into_iter()
                    .map(|frame| (seat, frame)),
            );
            frames
        });
        // A rejected command or a private seat setting can move the journal
        // without moving the opponent's view. Refresh its question/cursor too.
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            if !frames.iter().any(|(recipient, _)| *recipient == seat) {
                frames.extend(
                    self.session
                        .reask(seat)
                        .into_iter()
                        .map(|frame| (seat, frame)),
                );
            }
        }
        Ok(self.stage(
            step.position,
            step.command_id,
            request,
            &previous,
            accepted,
            frames,
        ))
    }

    fn stage(
        &mut self,
        index: u64,
        id: Vec<u8>,
        request: Hash,
        previous: &[u8],
        accepted: bool,
        frames: Frames,
    ) -> Vec<Envelope> {
        let asked_zero = self.session.asked_at(PlayerId::new(0));
        let asked_one = self.session.asked_at(PlayerId::new(1));
        let awaiting = u32::from(asked_zero.is_some()) | (u32::from(asked_one.is_some()) << 1);
        let outcome = match self.session.pending() {
            Pending::GameOver(result) => self
                .session
                .winning_seats(*result)
                .first()
                .map_or(3, |seat| u32::from(seat.get()) + 1),
            _ => 0,
        };
        // Full record bytes bind private inputs in addition to the engine's
        // compact replay checksum. Include both independently rebuilt views
        // and pending rules state; connection-specific delta caches stay local.
        let evidence = hash_fields(&[
            b"baylee/ranked/evidence/v1",
            previous,
            &request,
            &self.session.snapshot_hash().to_be_bytes(),
            &self.session.take_record(),
            &baylee_core::wire::encode(self.session.pending()),
            &baylee_core::wire::encode(&self.session.observe_view(PlayerId::new(0))),
            &baylee_core::wire::encode(&self.session.observe_view(PlayerId::new(1))),
            &baylee_core::wire::encode(&(accepted, asked_zero, asked_one, outcome)),
        ]);
        let vote = v1::RankedEngineVote {
            generation: self.generation,
            position: index,
            command_id: id,
            evidence: evidence.to_vec(),
            accepted,
            awaiting,
            outcome,
            asked_zero,
            asked_one,
        };
        let output = vec![wrap(Msg::RankedEngineVote(vote.clone()))];
        self.staged = Some((Position { request, vote }, frames));
        output
    }

    fn commit(&mut self, commit: &v1::RankedEngineCommit) -> Result<Vec<Envelope>, &'static str> {
        if commit.generation != self.generation {
            return Err("stale protected commit");
        }
        let chain: Hash = commit
            .chain
            .as_slice()
            .try_into()
            .map_err(|_| "invalid journal chain")?;
        if let Some(head) = &self.committed
            && head.position.vote.position == commit.position
        {
            if head.chain != chain || head.position.vote.evidence != commit.evidence {
                return Err("conflicting protected commit");
            }
            return Ok(vec![]);
        }
        let Some((next, _)) = &self.staged else {
            return Err("missing protected proposal");
        };
        if next.vote.position != commit.position || next.vote.evidence != commit.evidence {
            return Err("uncertified protected proposal");
        }
        let (position, frames) = self.staged.take().ok_or("missing protected proposal")?;
        self.committed = Some(Committed { position, chain });
        if self.primary && commit.publish {
            Ok(self.project(commit.position, chain, frames))
        } else {
            Ok(vec![])
        }
    }

    fn snapshot(
        &mut self,
        snapshot: &v1::RankedEngineSnapshot,
    ) -> Result<Vec<Envelope>, &'static str> {
        let head = self
            .committed
            .as_ref()
            .ok_or("uncommitted protected setup")?;
        if !self.primary
            || snapshot.generation != self.generation
            || snapshot.position != head.position.vote.position
            || snapshot.seat > 1
        {
            return Err("invalid protected snapshot");
        }
        // Until a tentative move has been certified, even a reconnect must
        // wait to read the session. The gateway reschedules it at commit.
        if self.staged.is_some() {
            return Ok(vec![]);
        }
        let seat = PlayerId::new(u8::try_from(snapshot.seat).map_err(|_| "invalid seat")?);
        // The gateway coalesces seat requests. A fresh socket must be able
        // to recover even after several reconnects at the same position.
        let chain = head.chain;
        let frames = self.whole(seat);
        Ok(self.project(snapshot.position, chain, frames))
    }

    fn whole(&mut self, seat: PlayerId) -> Frames {
        let mut frames = vec![self.session.game_static_envelope(seat), crate::curtain()];
        frames.extend(self.session.snapshot(seat));
        frames.into_iter().map(|frame| (seat, frame)).collect()
    }

    fn project(&self, position: u64, chain: Hash, frames: Frames) -> Vec<Envelope> {
        frames
            .into_iter()
            .map(|(seat, frame)| {
                let projection = v1::RankedSeatProjection {
                    generation: self.generation,
                    position,
                    chain: chain.to_vec(),
                    envelope: frame.encode_to_vec(),
                };
                crate::seat_frame(seat.get(), &wrap(Msg::RankedSeatProjection(projection)))
            })
            .collect()
    }
}
