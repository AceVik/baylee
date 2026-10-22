//! Goblin Grenadiers — {3}{R} — Creature — Goblin
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, destroy target creature and target land.
//! Set: WTH #104 — Weatherlight | Scryfall ID: 5a73db23-727f-4d63-97d7-2ca542276722 | Oracle ID: be83a518-cfab-46b1-9fe9-7e3d54311c37
// PARTIAL — Targets and destroys a creature when sacrificed on an unblocked attack; also targeting and destroying a land on the same triggered ability is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_GRENADIERS,
    oracle_id = "be83a518-cfab-46b1-9fe9-7e3d54311c37",
    scryfall_id = "5a73db23-727f-4d63-97d7-2ca542276722",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Targets and destroys a creature when sacrificed on an unblocked attack; also targeting and destroying a land on the same triggered ability is not yet modeled."
    ),
    faces = &[face!(
        name = "Goblin Grenadiers",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::Destroy {
                    target: TargetSpec::Object(&Filter::CREATURE),
                    no_regen: false,
                }],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
