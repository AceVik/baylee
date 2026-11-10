//! Deftblade Elite — {W} — Creature — Human Soldier
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Oracle: {1}{W}: Prevent all combat damage that would be dealt to and dealt by this creature this turn.
//! Set: VMA #23 — Vintage Masters | Scryfall ID: d576b8f2-9b5b-47c0-8f9f-faa209d59595 | Oracle ID: 9a3e3a8f-7c19-4b52-a1b4-08eb09f6127a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEFTBLADE_ELITE,
    oracle_id = "9a3e3a8f-7c19-4b52-a1b4-08eb09f6127a",
    scryfall_id = "d576b8f2-9b5b-47c0-8f9f-faa209d59595",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Deftblade Elite",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
