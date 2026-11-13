//! Sheltered Valley — (no cost) — Land
//! Oracle: If this land would enter, instead sacrifice each other permanent named Sheltered Valley you control, then put this land onto the battlefield.
//! Oracle: At the beginning of your upkeep, if you control three or fewer lands, you gain 1 life.
//! Oracle: {T}: Add {C}.
//! Set: ALL #142 — Alliances | Scryfall ID: 049d7a08-1605-4ce2-b8c5-634ce2a261e0 | Oracle ID: cd535fa3-6fd8-4227-97fd-3ef07cb0598d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHELTERED_VALLEY,
    oracle_id = "cd535fa3-6fd8-4227-97fd-3ef07cb0598d",
    scryfall_id = "049d7a08-1605-4ce2-b8c5-634ce2a261e0",
    faces = &[face!(
        name = "Sheltered Valley",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::SacrificeOthers(&Filter::And(&[
            Filter::ControlledByYou,
            Filter::Named("Sheltered Valley")
        ]))],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // The intervening condition is checked at trigger and resolution (CR 603.4).
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
            condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_LAND, 3)),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
