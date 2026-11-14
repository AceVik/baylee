//! Native live-table output held behind exact durable record acknowledgements.
use crate::{Clock, EngineRunner};
use baylee_engine::engine::NoLoopDetection;
use baylee_engine::extensions::NativeLookup;
use baylee_gamehost::RegistryLookup;
use baylee_protocol::v1::{self, Envelope};
use baylee_tower::{
    Refusal,
    cards::Catalog,
    v2::{execution::Prefix, package::Package},
};

mod barrier;
mod capture;
mod recovery;
#[cfg(test)]
mod tests;
use barrier::Barrier;
pub use barrier::Batch;
use capture::Capture;
pub use recovery::Recovery;

type Driver = EngineRunner<NoLoopDetection, NativeLookup<RegistryLookup, super::Pack>, Capture>;

/// Exact native game with no API that bypasses its persisted-output barrier.
/// The caller authenticates authority ACKs; this layer never contacts storage.
pub struct Table {
    driver: Driver,
    barrier: Barrier,
    refused: bool,
}

impl Table {
    /// Start a fresh private reservation. Recovery must reconstruct its retained
    /// prefix instead of invoking this constructor to reroll a game.
    /// # Errors
    /// Invalid package, current artifact mismatch, bad fence/budget or setup failure.
    pub fn new(
        game: String,
        package: &Package,
        catalog: &Catalog,
        fence: u64,
        byte_limit: u64,
        names: [String; 2],
    ) -> Result<Self, Refusal> {
        presentation(&game, &names)?;
        let identity = crate::variant::identity();
        let preset = package.preset(catalog, &identity, |card| {
            baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
        })?;
        let mut table = Self {
            driver: Driver::default().with_pump_budget(1),
            barrier: Barrier::new(package.digest(catalog)?, fence, byte_limit)?,
            refused: false,
        };
        let setup = v1::GameSetup {
            game_id: game,
            preset_json: serde_json::to_vec(&preset).map_err(|_| Refusal::Invalid)?,
            seat_names: names.into(),
            seat_guests: vec![false; 2],
            seat_accounts: vec![true, false],
            execution: identity.to_metadata(),
        };
        table.perform(|driver| {
            driver.handle(
                Envelope {
                    msg: Some(v1::envelope::Msg::GameSetup(setup)),
                },
                &[],
            )
        })?;
        if table.driver.session().is_none() {
            return Err(Refusal::Invalid);
        }
        Ok(table)
    }

    /// A fixed operation remains repeatable until its entire atomic commit is acknowledged.
    #[must_use]
    pub fn batch(&self) -> Option<Batch<'_>> {
        (!self.refused).then(|| self.barrier.batch()).flatten()
    }

    /// Release a batch only after every record chunk produced by it was committed.
    /// # Errors
    /// Another generation, fabricated/reordered head or failed technical boundary.
    pub fn acknowledge(&mut self, fence: u64, prefix: Prefix) -> Result<Vec<Envelope>, Refusal> {
        if self.refused {
            return Err(Refusal::Unavailable);
        }
        self.barrier.ack(fence, prefix)
    }

    /// Last acknowledged head for sealing/recovery. Never exposes speculative progress.
    #[must_use]
    pub fn acknowledged(&self) -> Prefix {
        self.barrier.prefix()
    }

    /// Accept a table input only when earlier effects are durably acknowledged.
    /// # Errors
    /// A pending ACK, refused boundary or exhausted bounded output batch.
    pub fn handle(
        &mut self,
        envelope: Envelope,
        remaining: &[(Clock, u32)],
    ) -> Result<Vec<Envelope>, Refusal> {
        self.perform(|driver| driver.handle(envelope, remaining))
    }

    /// One frozen Expert decision, with exactly the same persistence barrier.
    /// # Errors
    /// A pending ACK or technical allocation failure, never a game result.
    pub fn pump(&mut self, remaining: &[(Clock, u32)]) -> Result<Vec<Envelope>, Refusal> {
        self.perform(|driver| driver.pump_slice(remaining))
    }

    /// True only when the outer transport may schedule one Expert step.
    #[must_use]
    pub fn needs_pump(&self) -> bool {
        !self.refused && !self.barrier.waiting() && self.driver.needs_pump()
    }

    /// Supply the host clock only outside an unresolved append.
    /// # Errors
    /// The table is still awaiting durable storage or already refused.
    pub fn tell_time(&mut self, unix_ms: u64) -> Result<(), Refusal> {
        if self.refused || self.barrier.waiting() {
            return Err(Refusal::Unavailable);
        }
        // A reconnect or host clock correction cannot make accepted evidence
        // decrease its timestamps and become unverifiable after another restart.
        self.driver.tell_time(unix_ms.max(self.driver.now));
        Ok(())
    }

    /// Readiness deadline; storage waiting does not start the presentation timer.
    #[must_use]
    pub fn entrance_deadline(&self) -> Option<u64> {
        (!self.refused && !self.barrier.waiting())
            .then(|| self.driver.entrance_deadline())
            .flatten()
    }

    /// Complete a scheduled entrance through the same record/visibility boundary.
    /// # Errors
    /// A pending ACK or technical allocation failure.
    pub fn finish_entrance(&mut self) -> Result<Vec<Envelope>, Refusal> {
        self.perform(Driver::finish_entrance)
    }

    /// True only after terminal output has crossed the committed record barrier.
    #[must_use]
    pub fn finished(&self) -> bool {
        !self.refused && !self.barrier.waiting() && self.driver.finished()
    }

    fn perform(
        &mut self,
        operation: impl FnOnce(&mut Driver) -> Vec<Envelope>,
    ) -> Result<Vec<Envelope>, Refusal> {
        if self.refused || self.barrier.waiting() {
            return Err(Refusal::Unavailable);
        }
        let frames = operation(&mut self.driver);
        self.driver.capture_pending_record();
        let result = self
            .driver
            .records
            .take()
            .and_then(|pieces| self.barrier.stage(pieces, frames));
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

fn presentation(game: &str, names: &[String; 2]) -> Result<(), Refusal> {
    if game.is_empty()
        || game.len() > 128
        || game.chars().any(char::is_control)
        || names
            .iter()
            .any(|name| name.is_empty() || name.len() > 160 || name.chars().any(char::is_control))
    {
        Err(Refusal::Invalid)
    } else {
        Ok(())
    }
}
