//! Spymaster's Vault — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Swamp.
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}: Target creature you control connives X, where X is the number of creatures that died this turn. (Draw X cards, then discard X cards. Put a +1/+1 counter on that creature for each nonland card discarded this way.)
//! Set: MH3 #230 — Modern Horizons 3 | Scryfall ID: 3d5fbb30-abfc-4e79-8ce5-bbb04a241c9f | Oracle ID: 69ddca4b-5cc0-45f3-b2e6-a047c8d601be
// PARTIAL — the enters-tapped condition and {T}: Add {B} are built; the
// connive ability has no vocabulary and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::SPYMASTER_S_VAULT,
    oracle_id = "69ddca4b-5cc0-45f3-b2e6-a047c8d601be",
    scryfall_id = "3d5fbb30-abfc-4e79-8ce5-bbb04a241c9f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the connive ability: no Effect is a connive, and no Amount counts the \
         creatures that died this turn"
    ),
    faces = &[face!(
        name = "Spymaster's Vault",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::HasSubtype(
            land::SWAMP
        ))],
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "{B}, {T}: Target creature you control connives X, where X
// is the number of creatures that died this turn." — connive is not an
// `Effect` (nothing draws-then-discards-then-counters off what was
// discarded) and no `Amount` reads the creatures that died this turn, so the
// ability comes off the card rather than being written as something else.
