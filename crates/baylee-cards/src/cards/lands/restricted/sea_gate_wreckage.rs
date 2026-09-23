//! Sea Gate Wreckage — (no cost) — Land
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {2}{C}, {T}: Draw a card. Activate only if you have no cards in hand.
//! Set: CMM #1028 — Commander Masters | Scryfall ID: 1c137634-7531-4573-9bfa-623d8edf839c | Oracle ID: 91f34686-cb96-49c0-b4a7-49dd1fd076e2
// IMPLEMENTED — both halves; the draw is gated on the empty hand it prints.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEA_GATE_WRECKAGE,
    oracle_id = "91f34686-cb96-49c0-b4a7-49dd1fd076e2",
    scryfall_id = "1c137634-7531-4573-9bfa-623d8edf839c",
    faces = &[face!(name = "Sea Gate Wreckage", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // The {C} in the cost is a colourless *requirement* and not a
        // generic one: this land is the only source it brings to its own
        // price, which is why the cost is written `{2}{C}` and not `{3}`.
        activated!(
            cost!("{2}{C}", TapSelf),
            &[Effect::draw(1)],
            condition = Some(Condition::HandSizeAtMost(0)),
        ),
    ],
);
