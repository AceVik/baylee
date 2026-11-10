//! Avishkar Raceway — (no cost) — Land
//! Oracle: Start your engines! (If you have no speed, it starts at 1. It increases once on each of your turns when an opponent loses life. Max speed is 4.)
//! Oracle: {T}: Add {C}.
//! Oracle: Max speed — {3}, {T}, Discard a card: Draw a card.
//! Set: DFT #249 — Aetherdrift | Scryfall ID: 08a6b378-c7fa-4226-a310-4ee7e550b4d6 | Oracle ID: e2c35551-1ba5-4424-baf9-821b49bbcc8c
// IMPLEMENTED — persistent speed starts through the keyword; max speed unlocks the second activation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVISHKAR_RACEWAY,
    oracle_id = "e2c35551-1ba5-4424-baf9-821b49bbcc8c",
    scryfall_id = "08a6b378-c7fa-4226-a310-4ee7e550b4d6",
    faces = &[face!(name = "Avishkar Raceway", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf, Discard(&Filter::Any)),
            &[Effect::draw(1)],
            condition = Some(Condition::MaxSpeed)
        ),
        AbilityDef::StartYourEngines,
    ],
);
