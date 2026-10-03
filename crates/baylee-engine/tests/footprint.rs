//! Memory-footprint regression guard.
//!
//! `GameState::clone` is the AI lookahead primitive. Arena snapshots share
//! storage until a write copies it, so an object-changing ply still multiplies
//! the size of one [`GameObject`] by every object in the game. A field
//! added carelessly here is not a few bytes — it is a measurable slowdown
//! in every search the AI runs, and the number is invisible unless a test
//! prints it.
//!
//! These bounds are budgets, not natural constants. Raising one is a fine
//! thing to do deliberately; the test exists so it cannot happen by
//! accident. Update the number *and* `docs/perf-baseline.md` together.

use baylee_engine::object::{CachedChar, Characteristics, GameObject};
use baylee_engine::state::GameState;

/// Every characteristic set carries a 1024-bit subtype bitmap and a mana
/// cost (a count per symbol); those two dominate it and are what makes storing a second
/// copy per object expensive. Objects hold it behind an `Arc`, so this
/// number is paid once per *distinct* base, not once per object.
///
/// Raised 256 → 320 on 2026-09-19 by #43, which doubled the bitmap: 507 of
/// its 512 bits were assigned, and the widening had to happen before a set
/// forced it. The 64 bytes are deliberate and they are the reason the
/// `Arc` matters — measured on the same day, `GameObject` did not move at
/// all (272 B), so the AI's per-ply `GameState::clone` pays nothing for it.
const CHARACTERISTICS_BUDGET: usize = 320;

/// The projection cache holds a generation, an optional boxed projection
/// and an optional layer-2 controller — not a second `Characteristics`.
const CACHE_BUDGET: usize = 32;

/// One object: identity, zone, a handle on the base characteristics,
/// counters, riders, targets and the cache slot.
///
/// Went 528 → 272 when the *printed* characteristics moved behind an
/// `Arc`. They are written three times in the whole engine and read
/// everywhere, so inlining 256 bytes per object bought nothing and cost a
/// 256-byte memcpy per object on every `GameState::clone` — the AI's
/// per-ply primitive. Sharing also survives the copy: a token made from a
/// permanent points at the same base until something writes to it.
///
/// Raised 272 → 280 on 2026-09-23 by the second instance of the word
/// "target" (CR 115.3, a fight's two creatures). Inline it was two fields
/// and 32 bytes — the gate caught it at 304 — so it is one `Option<Box<…>>`,
/// null on every object but a spell or ability that says "target" twice,
/// and the eight bytes of that pointer are the whole cost.
///
/// Raised 280 → 288 on 2026-09-24 by the printed face an object's own
/// ability list is (`GameObject::own_face`), which a client needs to draw a
/// copy's sentence and which the list's address cannot supply. Packed into a
/// `NonZeroU32` so its `Option` is four bytes: the plain pair measured 296.
///
/// Raised 288 → 296 on 2026-09-29 by what was paid for a spell or ability
/// (`GameObject::paid`: the sacrificed creature's mana value, the mana
/// spent), which an effect reads back after the paid-for object has left
/// the battlefield. One `Option<Box<PaidRecord>>`, null on every object
/// that is not a paid-for spell or ability on the stack.
///
/// Raised 296 → 304 on 2026-09-29 by "that much" damage a triggering event
/// dealt (`GameObject::event_amount`, Questing Beast), carried onto the
/// triggered ability as it is put on the stack. The field is an
/// `Option<NonZeroU16>`, two bytes, and still cost eight: 296 was packed to
/// the byte, so any field at all rounds the object up by its alignment, and
/// a `Box` would cost the same eight. Folding it and `event_object` into one
/// boxed record would win them back.
///
/// Raised 296 → 304 on 2026-09-29 by the card name chosen as a permanent
/// entered (`GameObject::chosen_name`, Pithing Needle), which the lock reads
/// and a client shows. Packed like `own_face` into a `NonZeroU32`, so its
/// `Option` is four bytes, and the object grows by the eight its alignment
/// rounds them to: there was no four-byte hole left to put it in.
///
/// The two raises were made on two branches the same day, each from 296.
/// Merged, both fields sit in the one eight-byte step (six of its bytes),
/// and the object measured 304 with both.
///
/// Raised 304 → 312 on 2026-09-30 by `GameObject::controlled_since`, the
/// moment its controller took it (CR 302.6), split off `timestamp` so that
/// a transform can take the new timestamp CR 613.7g gives it without
/// making the permanent summoning-sick. It is a stamp on the game's `u64`
/// clock like the field it was split from, and there is no hole for it:
/// the eight-byte step the two fields above share has two bytes left.
const OBJECT_BUDGET: usize = 312;

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
