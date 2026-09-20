//! Haunted Fengraf — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice this land: Return a creature card at random from your graveyard to your hand.
//! Set: C18 #254 — Commander 2018 | Scryfall ID: 97a1d55a-39a8-4bf4-91b7-5565146c9c40 | Oracle ID: 7c6143f3-ad2c-4d7f-9041-aa59f01d8fb7
// IMPLEMENTED — {T}: Add {C}; the graveyard return is Partial (see below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAUNTED_FENGRAF,
    oracle_id = "7c6143f3-ad2c-4d7f-9041-aa59f01d8fb7",
    scryfall_id = "97a1d55a-39a8-4bf4-91b7-5565146c9c40",
    faces = &[face!(name = "Haunted Fengraf", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the {3} ability returns a creature card **at random** from your graveyard, \
         and the DSL has no random selection",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{3}, {T}, Sacrifice this land: Return a creature card
        // at random from your graveyard to your hand." — the whole activated
        // ability is expressible except the word "at random": Effect::
        // GraveyardToHand targets a card in a graveyard, which is a card the
        // caster chooses and not one picked at random, and no variant (nor
        // ReturnChosenToHand, which has each player pick) selects blindly.
    ],
);
