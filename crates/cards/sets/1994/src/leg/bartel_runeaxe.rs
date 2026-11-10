//! Bartel Runeaxe — {3}{B}{R}{G} — Legendary Creature — Giant Warrior
//! Oracle: Vigilance
//! Oracle: Bartel Runeaxe can't be the target of Aura spells.
//! Set: ME3 #145 — Masters Edition III | Scryfall ID: 3c635e33-c9f1-4508-986d-c0289921c299 | Oracle ID: 9beccf09-c024-408b-9f84-fe2c7462babb

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARTEL_RUNEAXE,
    oracle_id = "9beccf09-c024-408b-9f84-fe2c7462babb",
    scryfall_id = "3c635e33-c9f1-4508-986d-c0289921c299",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Bartel Runeaxe",
        mana_cost = mana!("{3}{B}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GIANT, subtypes::creature::WARRIOR],
        power = Some(6),
        toughness = Some(5),
    ),],
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBeTargetedBy(&Filter::And(&[
            Filter::InZone(ZoneRef::Stack),
            Filter::HasSubtype(subtypes::enchantment::AURA),
        ]))
    )],
);
