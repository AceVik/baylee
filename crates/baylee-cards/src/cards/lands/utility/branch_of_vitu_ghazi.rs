//! Branch of Vitu-Ghazi — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Disguise {3} (You may cast this card face down for {3} as a 2/2 creature with ward {2}. Turn it face up any time for its disguise cost.)
//! Oracle: When this land is turned face up, add two mana of any one color. Until end of turn, you don't lose this mana as steps and phases end.
//! Set: MKM #258 — Murders at Karlov Manor | Scryfall ID: 73a8169f-b858-47a5-9c76-2e7c50ad4ecd | Oracle ID: 7a30316b-dcd5-4a4b-b959-eecde7ca92e7
// PARTIAL — the {T}: Add {C} ability only; disguise and the turned-face-up
// mana rider are not expressible in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRANCH_OF_VITU_GHAZI,
    oracle_id = "7a30316b-dcd5-4a4b-b959-eecde7ca92e7",
    scryfall_id = "73a8169f-b858-47a5-9c76-2e7c50ad4ecd",
    faces = &[face!(name = "Branch of Vitu-Ghazi", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "disguise (face-down cast for {3}, ward {2}, turn face up) and the \
         turned-face-up mana rider are not expressible",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: Disguise {3} — no face-down/morph ability kind
        // exists, and its ward {2} would be the synthetic keyword trigger
        // of a face the engine cannot put onto the battlefield face down.
        // NOT SUPPORTED: When this land is turned face up, add two mana of
        // any one color. Until end of turn, you don't lose this mana as
        // steps and phases end. — no "turned face up" trigger exists, and
        // no effect keeps mana in a player's pool through a step.
    ],
);
