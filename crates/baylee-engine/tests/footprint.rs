//! Memory-footprint regression guard.
//!
//! `GameState::clone` is the AI lookahead primitive: every ply copies the
//! whole arena, so the size of one [`GameObject`] is multiplied by every
//! object in the game and again by every node the search visits. A field
//! added carelessly here is not a few bytes — it is a measurable slowdown
//! in every search the AI runs, and the number is invisible unless a test
//! prints it.
//!
//! These bounds are budgets, not natural constants. Raising one is a fine
//! thing to do deliberately; the test exists so it cannot happen by
//! accident. Update the number *and* `docs/perf-baseline.md` together.

use baylee_engine::object::{CachedChar, Characteristics, GameObject};
use baylee_engine::state::GameState;

/// Every characteristic set carries a 512-bit subtype bitmap and a 16-slot
/// mana cost; those two dominate it and are what makes storing a second
/// copy per object expensive.
const CHARACTERISTICS_BUDGET: usize = 256;

/// The projection cache holds a generation, an optional boxed projection
/// and an optional layer-2 controller — not a second `Characteristics`.
const CACHE_BUDGET: usize = 32;

/// One object: identity, zone, base characteristics, counters, riders,
/// targets and the cache slot.
const OBJECT_BUDGET: usize = 512;

#[test]
fn game_object_stays_within_its_budget() {
    let chars = size_of::<Characteristics>();
    let cache = size_of::<CachedChar>();
    let object = size_of::<GameObject>();
    let state = size_of::<GameState>();
    println!(
        "Characteristics = {chars} B\nCachedChar      = {cache} B\n\
         GameObject      = {object} B\nGameState       = {state} B"
    );
    assert!(
        chars <= CHARACTERISTICS_BUDGET,
        "Characteristics grew to {chars} B (budget {CHARACTERISTICS_BUDGET} B)"
    );
    assert!(
        cache <= CACHE_BUDGET,
        "the projection cache grew to {cache} B (budget {CACHE_BUDGET} B) — \
         it must stay a slot, not a second copy of the characteristics"
    );
    assert!(
        object <= OBJECT_BUDGET,
        "GameObject grew to {object} B (budget {OBJECT_BUDGET} B)"
    );
}

/// The cache is a *slot*: it must be strictly smaller than the value it
/// would otherwise inline, or storing it per object buys nothing.
#[test]
fn the_cache_is_smaller_than_the_value_it_replaces() {
    assert!(
        size_of::<CachedChar>() * 4 < size_of::<Characteristics>(),
        "an object with no layer effects on it should pay a pointer, not a projection"
    );
}
