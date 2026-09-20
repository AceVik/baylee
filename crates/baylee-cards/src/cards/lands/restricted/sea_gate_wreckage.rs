//! Sea Gate Wreckage — (no cost) — Land
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {2}{C}, {T}: Draw a card. Activate only if you have no cards in hand.
//! Set: CMM #1028 — Commander Masters | Scryfall ID: 1c137634-7531-4573-9bfa-623d8edf839c | Oracle ID: 91f34686-cb96-49c0-b4a7-49dd1fd076e2
// PARTIAL — {T}: Add {C} is built; the {2}{C}, {T} draw ability is dropped,
// because no Condition variant reads a player's hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEA_GATE_WRECKAGE,
    oracle_id = "91f34686-cb96-49c0-b4a7-49dd1fd076e2",
    scryfall_id = "1c137634-7531-4573-9bfa-623d8edf839c",
    faces = &[face!(name = "Sea Gate Wreckage", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {2}{C}, {T} ability prints \"Activate only if you have no cards in \
         hand\", and Condition has no hand-size variant — its five sentences are \
         ControlCount, OpponentGraveyardCountAtLeast, CountersOnSelf, \
         CountersOnSelfExactly and SourceMatches"
    ),
    abilities = &[
        // NOT SUPPORTED: "{2}{C}, {T}: Draw a card. Activate only if you have no
        // cards in hand." — no `Condition` variant reads the controller's hand,
        // and an ability offered unconditionally would draw cards the card does
        // not allow, so the ability comes off rather than shipping as a free
        // draw. The {{C}} half of the cost is not the blocker: `ManaColor` has
        // Colorless and `mana!("{2}{C}")` parses it.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
