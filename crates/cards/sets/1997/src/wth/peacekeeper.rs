//! Peacekeeper — {2}{W} — Creature — Human
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {1}{W}.
//! Oracle: Creatures can't attack.
//! Set: WTH #22 — Weatherlight | Scryfall ID: 592a5683-5f2f-4933-9fc3-5f7773f72f93 | Oracle ID: e82ce5c6-1e42-4e87-b696-67aa3822ec9c
// IMPLEMENTED — Upkeep sacrifice unless you pay {1}{W}, and creatures can't attack.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PEACEKEEPER,
    oracle_id = "e82ce5c6-1e42-4e87-b696-67aa3822ec9c",
    scryfall_id = "592a5683-5f2f-4933-9fc3-5f7773f72f93",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Peacekeeper",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{1}{W}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        static_ability!(Filter::CREATURE, Modifier::CantAttack),
    ],
);
