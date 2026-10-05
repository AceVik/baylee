//! Ur-Drago — {3}{U}{U}{B}{B} — Legendary Creature — Elemental
//! Oracle: First strike
//! Oracle: Creatures with swampwalk can be blocked as though they didn't have swampwalk.
//! Set: LEG #268 — Legends | Scryfall ID: 81a40f34-fc26-4d05-9c52-6ffbf1766a3b | Oracle ID: 537162d5-a2c2-4ad4-9fae-d2ec7a269dcc
// PARTIAL — first strike is on the card; the swampwalk shutdown is not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UR_DRAGO,
    oracle_id = "537162d5-a2c2-4ad4-9fae-d2ec7a269dcc",
    scryfall_id = "81a40f34-fc26-4d05-9c52-6ffbf1766a3b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ur-Drago",
        mana_cost = mana!("{3}{U}{U}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "no Modifier turns a landwalk keyword off for the blocking rule \
         alone; combat::can_block reads the keyword bits directly and \
         nothing suppresses them \"as though\""
    ),
    // NOT SUPPORTED: "Creatures with swampwalk can be blocked as though they
    // didn't have swampwalk." — no `Modifier` suppresses a landwalk keyword
    // for the declaration of blockers, and stripping `SWAMPWALK` from every
    // creature would be a different card.
);
