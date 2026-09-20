//! Choked Estuary — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Set: MSC #229 — Marvel Super Heroes Commander | Scryfall ID: 0c37e84e-e7e0-4dfe-bf17-daeb2e434861 | Oracle ID: d473b507-8c33-4118-bc10-b0a268776074
// PARTIAL — the mana line only: {T}: Add {U} or {B}.
// NOT SUPPORTED: "As this land enters, you may reveal an Island or Swamp card
// from your hand. If you don't, this land enters tapped." — the nearest
// EnterModifier, TappedUnless, asks about a permanent on the battlefield, and
// this card asks about a card in its controller's hand; nothing in the
// vocabulary reads a hand as a land enters.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHOKED_ESTUARY,
    oracle_id = "d473b507-8c33-4118-bc10-b0a268776074",
    scryfall_id = "0c37e84e-e7e0-4dfe-bf17-daeb2e434861",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "Choked Estuary", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the enter clause reveals an Island or Swamp card from your hand, and no EnterModifier reads a hand"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Black,
    ])])],
);
