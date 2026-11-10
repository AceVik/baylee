//! Skyshroud Ranger — {G} — Creature — Elf Ranger
//! Oracle: {T}: You may put a land card from your hand onto the battlefield. Activate only as a sorcery.
//! Set: 10E #297 — Tenth Edition | Scryfall ID: 9a2fd9e5-1986-4998-b08c-80897b9b27e7 | Oracle ID: 8f47a230-7500-4afb-97cd-dbd444c307c8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKYSHROUD_RANGER,
    oracle_id = "8f47a230-7500-4afb-97cd-dbd444c307c8",
    scryfall_id = "9a2fd9e5-1986-4998-b08c-80897b9b27e7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skyshroud Ranger",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::RANGER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::LAND,
            mana_value: None,
            optional: true,
        }],
        timing = ActivationTiming::SorcerySpeed,
    )],
);
