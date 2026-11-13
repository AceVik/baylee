//! Forsaken City — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may exile a card from your hand. If you do, untap this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: PLS #139 — Planeshift | Scryfall ID: 676703fe-bd80-413c-8704-1da5d3248b7e | Oracle ID: 6bb00a28-8b5a-4049-93b7-3db02de88aeb

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORSAKEN_CITY,
    oracle_id = "6bb00a28-8b5a-4049-93b7-3db02de88aeb",
    scryfall_id = "676703fe-bd80-413c-8704-1da5d3248b7e",
    faces = &[face!(name = "Forsaken City", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        mana_ability!(&[Effect::mana_of_any_color()]),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::PlayerMayPayCostThen {
                player: PlayerRel::You,
                cost: &CostPart::ExileFromHand(&Filter::Any),
                effect: &Effect::UntapSelf
            }]
        ),
    ],
);
