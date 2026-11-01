//! Merieke Ri Berit — {W}{U}{B} — Legendary Creature — Human
//! Oracle: Merieke Ri Berit doesn't untap during your untap step.
//! Oracle: {T}: Gain control of target creature for as long as you control Merieke Ri Berit. When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature. It can't be regenerated.
//! Set: TSB #95 — Time Spiral Timeshifted | Scryfall ID: 63f7cf94-78f6-4f30-ab24-8b90231520f1 | Oracle ID: 288fdfbb-1236-4732-afd7-e7eaba860007
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERIEKE_RI_BERIT,
    oracle_id = "288fdfbb-1236-4732-afd7-e7eaba860007",
    scryfall_id = "63f7cf94-78f6-4f30-ab24-8b90231520f1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Merieke Ri Berit",
        mana_cost = mana!("{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
