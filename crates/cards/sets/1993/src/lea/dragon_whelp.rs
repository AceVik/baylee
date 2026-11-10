//! Dragon Whelp — {2}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {R}: This creature gets +1/+0 until end of turn. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.
//! Set: PLST #DMR-116 — The List | Scryfall ID: 7f7bdfb7-1dc1-4ddf-8146-9276dff37b2b | Oracle ID: 705a1985-ed39-4a4b-812e-a677170b596e
// IMPLEMENTED — the count is of activations, taken as each is activated
// (`Effect::IfActivatedThisTurnAtLeast`); the delayed sacrifice is about the
// Whelp as it was when the ability resolved.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_WHELP,
    oracle_id = "705a1985-ed39-4a4b-812e-a677170b596e",
    scryfall_id = "7f7bdfb7-1dc1-4ddf-8146-9276dff37b2b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dragon Whelp",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[
            Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn
            },
            Effect::IfActivatedThisTurnAtLeast {
                n: 4,
                then: &[Effect::AtNextEndStep {
                    effects: &[Effect::SacrificeObject {
                        target: TargetSpec::EventObject
                    }]
                }]
            }
        ]
    ),],
);
