//! Magosi, the Waterveil — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {U}, {T}: Put an eon counter on this land. Skip your next turn.
//! Oracle: {T}, Remove an eon counter from this land and return it to its owner's hand: Take an extra turn after this one.
//! Set: ZEN #218 — Zendikar | Scryfall ID: 3c84cf70-4164-47ea-8da1-c0ca1ac132e1 | Oracle ID: 4bdffa67-e6b3-4588-b76e-c11db6f043ca
// PARTIAL — enters tapped and taps for {U}; the eon-counter pair is dropped.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "{U}, {T}: Put an eon counter on this land. Skip your next
// turn." — no `Effect` says "skip a turn", and an eon counter has no id in
// `baylee_cards_dsl::counters`; the ability is left off the card rather than
// written without the drawback that pays for it.
// NOT SUPPORTED: "{T}, Remove an eon counter from this land and return it to
// its owner's hand: Take an extra turn after this one." —
// `Effect::TakeExtraTurn` exists, but the counter it removes is the one the
// ability above never puts on, so nothing in this pool could ever pay it.

card!(
    index = index::MAGOSI_THE_WATERVEIL,
    oracle_id = "4bdffa67-e6b3-4588-b76e-c11db6f043ca",
    scryfall_id = "3c84cf70-4164-47ea-8da1-c0ca1ac132e1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Magosi, the Waterveil",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the eon-counter pair cannot be said: no effect skips a turn, and an eon counter has no id in `counters`"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
