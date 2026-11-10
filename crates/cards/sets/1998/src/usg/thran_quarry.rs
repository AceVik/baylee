//! Thran Quarry — (no cost) — Land
//! Oracle: At the beginning of the end step, if you control no creatures, sacrifice this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: USG #329 — Urza's Saga | Scryfall ID: 4b2d6c41-7d82-4062-a783-37d88536279c | Oracle ID: 57b4da3f-361a-4cbe-b77f-190ec33eefd8
// IMPLEMENTED — the any-colour mana and the end-step sacrifice; Glimmervoid
// is the same land one card type over and is written the same way.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRAN_QUARRY,
    oracle_id = "57b4da3f-361a-4cbe-b77f-190ec33eefd8",
    scryfall_id = "4b2d6c41-7d82-4062-a783-37d88536279c",
    faces = &[face!(name = "Thran Quarry", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_of_any_color()]),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeSelf],
            condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_CREATURE, 0)),
        ),
    ],
);
