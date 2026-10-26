//! Delraich — {6}{B} — Creature — Horror
//! Oracle: You may sacrifice three black creatures rather than pay this spell's mana cost.
//! Oracle: Trample
//! Set: MMQ #133 — Mercadian Masques | Scryfall ID: da64094f-df6e-4c43-b4ae-03aab6b92816 | Oracle ID: 38f9e766-a2ad-437b-a38c-d6f3ffd261fb
// NOT SUPPORTED: Sacrificing three black creatures rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Delraich is cast for {6}{B} as a 6/6 Horror with trample.
// PARTIAL — Sacrificing three black creatures rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Delraich is cast for {6}{B} as a 6/6 Horror with trample.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DELRAICH,
    oracle_id = "38f9e766-a2ad-437b-a38c-d6f3ffd261fb",
    scryfall_id = "da64094f-df6e-4c43-b4ae-03aab6b92816",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "Sacrificing three black creatures rather than paying this spell's mana cost is not yet supported by cast_wizard::paid_as_an_alternative_cost; in play, Delraich is cast for {6}{B} as a 6/6 Horror with trample.",
    ),
    faces = &[face!(
        name = "Delraich",
        mana_cost = mana!("{6}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(6),
        toughness = Some(6),
    ),],
);
