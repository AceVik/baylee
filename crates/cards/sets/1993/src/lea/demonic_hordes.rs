//! Demonic Hordes — {3}{B}{B}{B} — Creature — Demon
//! Oracle: {T}: Destroy target land.
//! Oracle: At the beginning of your upkeep, unless you pay {B}{B}{B}, tap this creature and sacrifice a land of an opponent's choice.
//! Set: ME4 #76 — Masters Edition IV | Scryfall ID: dfd1442d-ac66-4475-8704-1eaaa76f4365 | Oracle ID: 2847c8a0-f6aa-4e4a-a7b8-fc116436a264
// IMPLEMENTED — land destruction and upkeep payment or opponent-chosen sacrifice.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEMONIC_HORDES,
    oracle_id = "2847c8a0-f6aa-4e4a-a7b8-fc116436a264",
    scryfall_id = "dfd1442d-ac66-4475-8704-1eaaa76f4365",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Demonic Hordes",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
            target = Some(TargetSpec::Object(&Filter::LAND))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{B}{B}{B}"),
                effect: &Effect::Sequence(&[
                    Effect::TapSelf,
                    Effect::SacrificeChosenByOpponent {
                        player: PlayerRel::You,
                        filter: &Filter::LAND,
                    },
                ]),
            }],
        ),
    ],
);
