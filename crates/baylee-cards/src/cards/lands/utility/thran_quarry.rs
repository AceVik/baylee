//! Thran Quarry — (no cost) — Land
//! Oracle: At the beginning of the end step, if you control no creatures, sacrifice this land.
//! Oracle: {T}: Add one mana of any color.
//! Set: USG #329 — Urza's Saga | Scryfall ID: 4b2d6c41-7d82-4062-a783-37d88536279c | Oracle ID: 57b4da3f-361a-4cbe-b77f-190ec33eefd8
// PARTIAL — {T}: Add one mana of any color. The end-step sacrifice comes off
// the card: its intervening-`if` cannot be said, see the NOT SUPPORTED line.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRAN_QUARRY,
    oracle_id = "57b4da3f-361a-4cbe-b77f-190ec33eefd8",
    scryfall_id = "4b2d6c41-7d82-4062-a783-37d88536279c",
    faces = &[face!(name = "Thran Quarry", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the end-step clause 'if you control no creatures': no Condition variant counts downwards, ControlCount reads only 'at least N'"
    ),
    abilities = &[
        // NOT SUPPORTED: At the beginning of the end step, if you control no
        // creatures, sacrifice this land. The clause is an intervening-`if`
        // (CR 603.4) and the sentence is a *negative* count — zero creatures
        // — while `Condition::ControlCount(&filter, n)` asks for at least n.
        // A trigger without the condition would sacrifice the land under a
        // controller who has creatures, so the ability is dropped rather
        // than written wrong.
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
);
