//! Isolated Watchtower — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Scry 1, then you may reveal the top card of your library. If a basic land card is revealed this way, put it onto the battlefield tapped. Activate only if an opponent controls at least two more lands than you.
//! Set: C18 #59 — Commander 2018 | Scryfall ID: 9ee3c4f0-ec6b-48cd-b1be-4620aea337d6 | Oracle ID: 3893f320-47fd-49ec-a78c-80bfb607a279
// PARTIAL — {T}: Add {C} implemented; the {2}, {T} scry/reveal ability is not
// expressible in the DSL (see the // NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ISOLATED_WATCHTOWER,
    oracle_id = "3893f320-47fd-49ec-a78c-80bfb607a279",
    scryfall_id = "9ee3c4f0-ec6b-48cd-b1be-4620aea337d6",
    faces = &[face!(name = "Isolated Watchtower", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the {2}, {T} ability needs an activation condition comparing an \
         opponent's land count to yours and an effect that reveals the top \
         card and puts a revealed basic land onto the battlefield tapped, \
         neither of which the DSL has"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}: Scry 1, then you may reveal the top card of
        // your library. If a basic land card is revealed this way, put it onto
        // the battlefield tapped. Activate only if an opponent controls at
        // least two more lands than you." — no `Condition` compares an
        // opponent's land count to yours, and no `Effect` reveals the top card
        // and conditionally puts a basic land onto the battlefield tapped.
    ],
);
