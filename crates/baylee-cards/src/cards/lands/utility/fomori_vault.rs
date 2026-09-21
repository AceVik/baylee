//! Fomori Vault — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Discard a card: Look at the top X cards of your library, where X is the number of artifacts you control. Put one of those cards into your hand and the rest on the bottom of your library in a random order.
//! Set: BIG #29 — The Big Score | Scryfall ID: a5433b98-4657-4bbe-9e72-d3c94c6aa8ef | Oracle ID: 622a53bd-d894-422c-a606-7126041afa02
// IMPLEMENTED — the {T}: Add {C} mana ability. The second ability is NOT
// SUPPORTED: its X is a computed count (artifacts you control) and
// Effect::LookAtTopPick takes a fixed u8 count, so the clause cannot be said.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOMORI_VAULT,
    oracle_id = "622a53bd-d894-422c-a606-7126041afa02",
    scryfall_id = "a5433b98-4657-4bbe-9e72-d3c94c6aa8ef",
    faces = &[face!(name = "Fomori Vault", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability looks at X cards where X is a computed count and LookAtTopPick takes a fixed u8"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {3}, {T}, Discard a card: Look at the top X cards of your library, where X is the number of artifacts you control. Put one of those cards into your hand and the rest on the bottom of your library in a random order.
    ],
);
