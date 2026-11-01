//! Orcish Farmer — {1}{R}{R} — Creature — Orc
//! Oracle: {T}: Target land becomes a Swamp until its controller's next untap step.
//! Set: ME2 #141 — Masters Edition II | Scryfall ID: 2563a5b5-ea7a-4400-9e40-292623db2f96 | Oracle ID: c3039d19-8c98-4953-8943-9922b6ab45ef
// PARTIAL — body only; the optional land-type change cannot expire before
// its controller determines which permanents untap in their next untap step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORCISH_FARMER,
    oracle_id = "c3039d19-8c98-4953-8943-9922b6ab45ef",
    scryfall_id = "2563a5b5-ea7a-4400-9e40-292623db2f96",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "body only; the optional land-type change is omitted because the affected-controller untap duration expires after untapping rather than before that step's untap determination",
    ),
    faces = &[face!(
        name = "Orcish Farmer",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: ending the land-type change before the affected land's
    // controller determines which permanents untap in their next untap step.
    // UntilAffectedUntapSteps(1) instead lasts through that determination,
    // allowing an Island changed into a Swamp to untap despite Choke.
);
