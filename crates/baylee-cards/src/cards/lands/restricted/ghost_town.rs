//! Ghost Town — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {0}: Return this land to its owner's hand. Activate only if it's not your turn.
//! Set: TMP #318 — Tempest | Scryfall ID: 4218cdda-3a62-43fb-aaf7-7ac836392796 | Oracle ID: f2c861d3-b302-4e84-b647-099551007269
// PARTIAL — {T}: Add {C} and the {0} self-bounce are built; the printed
// "Activate only if it's not your turn" has no `Condition` to hold it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHOST_TOWN,
    oracle_id = "f2c861d3-b302-4e84-b647-099551007269",
    scryfall_id = "4218cdda-3a62-43fb-aaf7-7ac836392796",
    faces = &[face!(name = "Ghost Town", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "Condition has no \"not your turn\" sentence, so {0}: Return this land to its owner's hand is offered on its controller's turn as well"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Activate only if it's not your turn." — the five
        // sentences Condition knows are all about permanents, counters and
        // graveyards; none of them can read whose turn it is, so this
        // ability activates unconditionally.
        activated!(
            cost!("{0}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject
            }]
        ),
    ],
);
