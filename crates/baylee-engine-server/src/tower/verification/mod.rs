//! Native Tower adjudication, including the authority's exact chunk commitment.
use baylee_gamehost::session::tower::{Refused, ReplayBudget};
use baylee_tower::{
    cards::Catalog,
    v2::{
        package::Package,
        run::{Binding, Outcome, Settlement},
    },
};
use std::{io::Read, time::Instant};

mod stream;
mod worker;
pub use worker::worker_entry;

/// Rebuild and verify the authority's package with this compiled Tower executable.
/// The stream uses the shared bounded worker frame: big-endian u32 length + raw bytes,
/// followed by EOF. It is private worker IPC, never a player-authenticated API.
/// The supervising worker must bound process memory, wall time and input delivery.
/// # Errors
/// Wrong artifact/package/commitment, malformed or illegal record, altered Expert
/// answers, incomplete termination or technical resource exhaustion.
pub fn verify_record(
    package: &Package,
    catalog: &Catalog,
    binding: Binding,
    source: impl Read,
    resources: ReplayBudget,
    deadline: Instant,
) -> Result<Settlement, Refused> {
    if binding.prefix.bytes > resources.bytes as u64 || Instant::now() >= deadline {
        return Err(Refused::Budget);
    }
    let identity = crate::variant::identity();
    if package.digest(catalog).map_err(|_| Refused::Invalid)? != binding.package {
        return Err(Refused::Invalid);
    }
    let preset = package
        .preset(catalog, &identity, |card| {
            baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
        })
        .map_err(|_| Refused::Invalid)?;
    let source = stream::Committed::new(source, binding)?;
    let verified = baylee_gamehost::session::tower::verify_native::<crate::variant::Registry>(
        &preset, &identity, source, resources, deadline,
    )?;
    let outcome = match verified.result.winner {
        Some(baylee_engine::win::Victor::Player(player)) if player.get() == 0 => Outcome::Victory,
        Some(baylee_engine::win::Victor::Player(player)) if player.get() == 1 => Outcome::Defeat,
        None => Outcome::Draw,
        _ => return Err(Refused::Invalid),
    };
    Ok(Settlement {
        binding,
        outcome,
        turns: verified.turn,
        used: verified.consumed[0],
    })
}

#[cfg(test)]
mod tests;
