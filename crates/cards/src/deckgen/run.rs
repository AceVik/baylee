//! A limited Commander adapter to the shared synergy/curve/mana-base builder.
use super::{Archetype, DeckBuildRequest, DeckGenError, Generated, Pool, coverage};
use crate::formats::Shape;
use baylee_core::ids::CardIndex;

/// Build exactly 39 library cards and one explicit commander. This is a Tower
/// formation, not a legal hundred-card Commander deck. Singleton, identity,
/// curve, functional quotas and mana sources use the ordinary builder.
/// # Errors
/// Missing/ineligible/colourless anchor, insufficient pool or illegal formation.
pub fn build_commander_run(
    pool: &Pool,
    commander: CardIndex,
    archetype: Archetype,
    seed: u64,
) -> Result<Generated, DeckGenError> {
    build_commander_run_with_core(pool, commander, archetype, seed, &[])
}

/// The same limited builder with up to twelve mandatory on-theme spells. These
/// seed the synergy accumulator and cannot be removed by local refinement.
/// # Errors
/// An invalid commander/package, missing pool card or illegal final formation.
pub fn build_commander_run_with_core(
    pool: &Pool,
    commander: CardIndex,
    archetype: Archetype,
    seed: u64,
    required: &[CardIndex],
) -> Result<Generated, DeckGenError> {
    if required.len() > 12 {
        return Err(DeckGenError(
            "run spell package exceeds twelve cards".into(),
        ));
    }
    if !pool
        .entries
        .iter()
        .any(|entry| entry.card == commander && entry.leader && !entry.identity.is_empty())
    {
        return Err(DeckGenError(
            "run commander is not in the admitted pool".into(),
        ));
    }
    let mut request = DeckBuildRequest::new(Shape::Commander, archetype, seed);
    request.anchor = Some(commander);
    let deck = coverage::build_sized(pool, &request, Some(40), required)?;
    if deck.leaders != [commander] {
        return Err(DeckGenError("run commander was substituted".into()));
    }
    Ok(deck)
}
