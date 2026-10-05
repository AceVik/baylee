//! Martyrs of Korlis — {3}{W}{W} — Creature — Human
//! Oracle: As long as this creature is untapped, all damage that would be dealt to you by artifacts is dealt to this creature instead.
//! Set: ME4 #20 — Masters Edition IV | Scryfall ID: 41c0b6a9-8d1b-42b6-862d-7b21023df3a7 | Oracle ID: 7ca54a23-f8eb-4982-b4ee-7392e2f2a1b3
// IMPLEMENTED — while untapped, artifact damage to you is dealt to this creature instead.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MARTYRS_OF_KORLIS,
    oracle_id = "7ca54a23-f8eb-4982-b4ee-7392e2f2a1b3",
    scryfall_id = "41c0b6a9-8d1b-42b6-862d-7b21023df3a7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Martyrs of Korlis",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    // "As long as this creature is untapped" is read on the permanent the
    // redirection applies to, as the damage would be dealt, so a Martyrs
    // tapped a moment earlier in the same resolution is already out of it
    // (Veteran Bodyguard's shape). The source filter is artifacts, so their
    // combat and noncombat damage are redirected alike.
    abilities = &[static_ability!(
        Filter::And(&[Filter::This, Filter::Untapped]),
        Modifier::RedirectDamageToYou(&Filter::ARTIFACT)
    )],
);
