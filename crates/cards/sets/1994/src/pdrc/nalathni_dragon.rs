//! Nalathni Dragon — {2}{R}{R} — Creature — Dragon
//! Oracle: Flying; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Oracle: {R}: This creature gets +1/+0 until end of turn. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.
//! Set: MB2 #197 — Mystery Booster 2 | Scryfall ID: 68005c8d-f047-4ee3-99cc-e7cd46feef60 | Oracle ID: 0887dc95-3e75-4f72-bf05-4067d3785376

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NALATHNI_DRAGON,
    oracle_id = "0887dc95-3e75-4f72-bf05-4067d3785376",
    scryfall_id = "68005c8d-f047-4ee3-99cc-e7cd46feef60",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLYING.union(KeywordSet::BANDING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nalathni Dragon",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(1),
        toughness = Some(1),
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
                duration: Duration::UntilEndOfTurn,
            },
            Effect::IfActivatedThisTurnAtLeast {
                n: 4,
                then: &[Effect::AtNextEndStep {
                    effects: &[Effect::SacrificeObject {
                        target: TargetSpec::EventObject,
                    }],
                }],
            },
        ]
    ),],
);
