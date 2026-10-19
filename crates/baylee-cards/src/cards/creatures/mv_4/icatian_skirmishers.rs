//! Icatian Skirmishers — {3}{W} — Creature — Human Soldier
//! Oracle: First strike; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: Whenever this creature attacks, all creatures banded with it gain first strike until end of turn.
//! Set: FEM #14 — Fallen Empires | Scryfall ID: 15f6d115-c02d-45a3-aa6d-402964df47dd | Oracle ID: 8ddbe82b-7fc9-4b2c-85de-dc747adbb3ca
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICATIAN_SKIRMISHERS,
    oracle_id = "8ddbe82b-7fc9-4b2c-85de-dc747adbb3ca",
    scryfall_id = "15f6d115-c02d-45a3-aa6d-402964df47dd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FIRST_STRIKE.union(KeywordSet::BANDING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Icatian Skirmishers",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::continuous(
            &Filter::BandedWithSource,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
            Duration::UntilEndOfTurn,
        )],
    ),],
);
