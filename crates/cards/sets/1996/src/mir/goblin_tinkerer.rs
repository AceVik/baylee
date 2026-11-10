//! Goblin Tinkerer — {1}{R} — Creature — Goblin Artificer
//! Oracle: {R}, {T}: Destroy target artifact. That artifact deals damage equal to its mana value to this creature.
//! Set: MIR #180 — Mirage | Scryfall ID: e6529852-8b3e-4a70-a4a1-029e012231c6 | Oracle ID: 0afc2d2f-4fdb-440f-9c35-45bf103c0437
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_TINKERER,
    oracle_id = "0afc2d2f-4fdb-440f-9c35-45bf103c0437",
    scryfall_id = "e6529852-8b3e-4a70-a4a1-029e012231c6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Tinkerer",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf),
        &[
            // "That artifact deals damage equal to its mana value to this
            // creature": the artifact is the source. It is read while it is
            // still on the battlefield, so it runs before the destroy; nothing
            // can happen between the two halves of one resolution.
            Effect::ForEachMatching {
                filter: &Filter::Reference,
                effects: &[Effect::EventObjectDealsDamage {
                    amount: Amount::TargetCmc,
                    target: TargetSpec::ThisObject,
                }],
            },
            Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT)),
        ],
        target = Some(TargetSpec::Object(&Filter::ARTIFACT)),
    )],
);
