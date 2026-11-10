//! Glimmervoid — (no cost) — Land
//! Oracle: At the beginning of the end step, if you control no artifacts, sacrifice this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: 2XM #319 — Double Masters | Scryfall ID: 4a639687-d9e3-46a8-bc9f-6ca3912c46ab | Oracle ID: b92e9854-4527-4133-8615-e282a213e7e3
// IMPLEMENTED — the any-colour mana, and the end-step sacrifice behind the
// downward count `Condition::ControlCountAtMost` now states.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLIMMERVOID,
    oracle_id = "b92e9854-4527-4133-8615-e282a213e7e3",
    scryfall_id = "4a639687-d9e3-46a8-bc9f-6ca3912c46ab",
    faces = &[face!(name = "Glimmervoid", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_of_any_color()]),
        // "At the beginning of the end step" and not "your end step":
        // `PlayerRel::EachPlayer` is the difference between a land that
        // checks itself once a turn cycle and one that checks itself once
        // per turn, which on a four-player table is four times as often.
        //
        // The clause is an intervening `if` (CR 603.4), so it is read twice
        // — when the ability would trigger and again as it resolves — and an
        // artifact that arrives in between saves the land.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeSelf],
            condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_ARTIFACT, 0)),
        ),
    ],
);
