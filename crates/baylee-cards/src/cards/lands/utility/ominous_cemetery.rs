//! Ominous Cemetery — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Exile this land: Target creature's owner shuffles it into their library.
//! Set: WHO #189 — Doctor Who | Scryfall ID: 2e843c57-fae3-4127-94e7-cad8c8bb9486 | Oracle ID: d002391f-1dad-4966-ac36-56cc3ec015b2
// IMPLEMENTED — {T}: Add {C}. The second ability is left off: the DSL has no
// effect that shuffles a permanent into its owner's library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OMINOUS_CEMETERY,
    oracle_id = "d002391f-1dad-4966-ac36-56cc3ec015b2",
    scryfall_id = "2e843c57-fae3-4127-94e7-cad8c8bb9486",
    faces = &[face!(name = "Ominous Cemetery", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {5}, {T}, Exile this land ability — no Effect shuffles a creature into its owner's library"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{5}, {T}, Exile this land: Target creature's owner shuffles
// it into their library." No `Effect` moves a battlefield permanent into a
// library at a random position. `Effect::PutTargetOnBottomOfLibrary` is the
// nearest variant and it puts the card on the *bottom* — a different sentence
// with a different result — and `Effect::ShuffleGraveyardIntoLibrary` shuffles
// a graveyard, not a permanent. The ability comes off the card rather than
// being approximated.
