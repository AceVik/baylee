//! Temple of the Dragon Queen — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Dragon card from your hand. This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.
//! Oracle: As this land enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Set: TDC #104 — Tarkir: Dragonstorm Commander | Scryfall ID: 91658f56-12c9-4173-94ad-dfd186b1dbae | Oracle ID: 169a26d2-7bc9-4403-9c92-98d4bd5ca4f3
// PARTIAL — the chosen colour (EnterModifier::ChooseColor plus
// Effect::mana_chosen) and the enters-tapped check against a Dragon you
// control, which is the half of the entry condition the DSL can say.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// NOT SUPPORTED: "you may reveal a Dragon card from your hand" — no
// `EnterModifier` reveals a card from hand, so the condition is read as
// "unless you control a Dragon" alone and a Dragon held in hand does not
// save the land from entering tapped.

card!(
    index = index::TEMPLE_OF_THE_DRAGON_QUEEN,
    oracle_id = "169a26d2-7bc9-4403-9c92-98d4bd5ca4f3",
    scryfall_id = "91658f56-12c9-4173-94ad-dfd186b1dbae",
    faces = &[face!(
        name = "Temple of the Dragon Queen",
        types = TypeSet::LAND,
        enter_modifiers = &[
            EnterModifier::TappedUnless(&f!(your Filter::HasSubtype(creature::DRAGON))),
            EnterModifier::ChooseColor,
        ],
    ),],
    coverage = Coverage::Partial(
        "the enters-tapped condition drops \"you may reveal a Dragon card from your hand\" — no EnterModifier reveals a card from hand",
    ),
    abilities = &[mana_ability!(&[Effect::mana_chosen()])],
);
