//! Exact retained-prefix recovery under a newly authorized assignment generation.
use super::{Barrier, Catalog, Driver, Package, Refusal, Table, presentation};
use baylee_engine::choice::Pending;
use baylee_gamehost::session::tower::{Refused, ReplayBudget, resume_native};
use baylee_tower::v2::run::Binding;
use std::{io::Read, time::Instant};

/// Private authority evidence. The caller must authenticate the fresh assignment
/// and supervise process memory/deadlines before passing a retained stream here.
pub struct Recovery<S> {
    /// Exact package/prefix and the authority's current assignment fence.
    pub binding: Binding,
    /// Original execution allocation, never increased by a reconnect.
    pub byte_limit: u64,
    /// Bounded big-endian length/raw-chunk stream, terminated by EOF.
    pub source: S,
    /// Operator work allocation, not a rules limit.
    pub resources: ReplayBudget,
    /// Cooperative deadline; a process supervisor handles nonreturning rules calls.
    pub deadline: Instant,
}

impl Table {
    /// Recover without rerolling, duplicating a header or resetting the Expert RNG.
    /// No frame is returned before the entire retained commitment is checked. A
    /// live game requires a fresh presentation-ready handshake; a committed
    /// terminal game is immediately ready for sealing/independent verification.
    /// # Errors
    /// Altered/truncated/incompatible evidence, invalid assignment or exhausted work.
    pub fn resume(
        game: String,
        package: &Package,
        catalog: &Catalog,
        names: [String; 2],
        recovery: Recovery<impl Read>,
    ) -> Result<Self, Refusal> {
        presentation(&game, &names)?;
        let Recovery {
            binding,
            byte_limit,
            source,
            resources,
            deadline,
        } = recovery;
        if binding.prefix.bytes > resources.bytes as u64 || Instant::now() >= deadline {
            return Err(Refusal::Capacity);
        }
        if package.digest(catalog)? != binding.package {
            return Err(Refusal::Invalid);
        }
        let barrier =
            Barrier::restored(binding.package, binding.fence, byte_limit, binding.prefix)?;
        let identity = crate::variant::identity();
        let preset = package.preset(catalog, &identity, |card| {
            baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
        })?;
        let source =
            super::super::verification::Committed::new(source, binding).map_err(refusal)?;
        let restored =
            resume_native(&preset, &identity, source, resources, deadline).map_err(refusal)?;
        let now = restored.at();
        let mut session = restored.into_session(game.clone(), names.into());
        session.mark_guests(vec![false; 2]);
        session.mark_people(&[true, false]);
        #[cfg(test)]
        session.tell_views_whole();
        let ended = matches!(session.pending(), Pending::GameOver(_));
        let waits_for = session.human_seats().into_iter().collect();
        let driver = Driver {
            session: Some(session),
            game_id: game,
            now,
            ended,
            record_seq: u32::try_from(binding.prefix.chunks).map_err(|_| Refusal::Invalid)?,
            record_sent_at: now,
            curtain: (!ended).then_some(crate::Barrier {
                waits_for,
                ready: baylee_core::ids::SeatSet::new(),
            }),
            ..Driver::default().with_pump_budget(1)
        };
        Ok(Self {
            driver,
            barrier,
            refused: false,
        })
    }
}

fn refusal(error: Refused) -> Refusal {
    match error {
        Refused::Invalid => Refusal::Invalid,
        Refused::Budget => Refusal::Capacity,
    }
}
