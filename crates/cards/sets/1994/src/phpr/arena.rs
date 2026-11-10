//! Arena — (no cost) — Land
//! Oracle: {3}, {T}: Tap target creature you control and target creature of an opponent's choice they control. Those creatures fight each other. (Each deals damage equal to its power to the other.)
//! Set: TSB #117 — Time Spiral Timeshifted | Scryfall ID: e5b7afa9-e07a-4a84-a41f-7a8bc5c1d274 | Oracle ID: 12326d23-1fb5-4084-a23a-d6e2c90ea36c
// IMPLEMENTED — {3}, {T}: tap target creature you control and target creature of an opponent's choice they control; those creatures fight each other.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARENA,
    oracle_id = "12326d23-1fb5-4084-a23a-d6e2c90ea36c",
    scryfall_id = "e5b7afa9-e07a-4a84-a41f-7a8bc5c1d274",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Arena", types = TypeSet::LAND,),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::TapTarget,
            Effect::Fight {
                fighter: TargetSlot::First,
                foe: TargetSlot::Second,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        second_targets = Some(
            TargetReq::one(TargetSpec::Object(&Filter::OPPONENT_CREATURE)).chosen_by_opponent()
        ),
    ),],
);
