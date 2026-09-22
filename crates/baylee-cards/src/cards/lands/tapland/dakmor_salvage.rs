//! Dakmor Salvage — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: Dredge 2 (If you would draw a card, you may mill two cards instead. If you do, return this card from your graveyard to your hand.)
//! Set: EOC #156 — Edge of Eternities Commander | Scryfall ID: 72474b66-1dda-409d-8703-b026c1777081 | Oracle ID: cdc4048a-73ec-4ec1-a179-2b36c397bf1a
// PARTIAL — enters tapped (EnterModifier::Tapped) and taps for {B}; dredge 2
// has no DSL spelling.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAKMOR_SALVAGE,
    oracle_id = "cdc4048a-73ec-4ec1-a179-2b36c397bf1a",
    scryfall_id = "72474b66-1dda-409d-8703-b026c1777081",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dakmor Salvage",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial("dredge 2 — no DSL variant replaces a draw"),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "Dredge 2 (If you would draw a card, you may mill two cards
// instead. If you do, return this card from your graveyard to your hand.)" —
// no `ReplacementRule` replaces a draw (the enum carries token and counter
// doubling and the two trigger modifiers), and no ability is activatable from
// a graveyard, so the return half has no door either.
