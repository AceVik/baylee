//! Sequestered Stash — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Sacrifice this land: Mill five cards. Then you may put an artifact card from your graveyard on top of your library.
//! Set: KLD #248 — Kaladesh | Scryfall ID: 86a17084-bb96-4e81-bff0-005bd44a1fbd | Oracle ID: b6fe779f-b20d-49cc-96dd-54f1ffb312e1
// PARTIAL — the {C} mana ability and the {4}, {T}, sacrifice mill-five are
// built; the "then you may put an artifact card from your graveyard on top
// of your library" half is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEQUESTERED_STASH,
    oracle_id = "b6fe779f-b20d-49cc-96dd-54f1ffb312e1",
    scryfall_id = "86a17084-bb96-4e81-bff0-005bd44a1fbd",
    faces = &[face!(name = "Sequestered Stash", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"Then you may put an artifact card from your graveyard on top of your library\" \
         is not expressible: the only graveyard-to-library effects (Effect::GraveyardToTop) \
         read a TargetSpec, so their card is chosen as the ability is activated — before the \
         mill runs — and could never be one of the five cards it puts there. No non-targeted \
         chooser from a graveyard exists."
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Then you may put an artifact card from your
        // graveyard on top of your library." — Effect::GraveyardToTop is
        // Volrath's Stronghold's *targeted* sentence.
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::Mill {
                amount: Amount::Fixed(5),
                target: PlayerRel::You,
            }]
        ),
    ],
);
