//! Necroblossom Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Swamp or Forest card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Set: SOC #389 — Secrets of Strixhaven Commander | Scryfall ID: 4355ba23-de6a-4f18-baf4-82ae47cc2965 | Oracle ID: 761ee6f9-b0fa-43c9-8d1f-9591ea18e52d
// PARTIAL — the mana ability is the whole of what is built; the as-it-enters
// reveal clause has no `EnterModifier` variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECROBLOSSOM_SNARL,
    oracle_id = "761ee6f9-b0fa-43c9-8d1f-9591ea18e52d",
    scryfall_id = "4355ba23-de6a-4f18-baf4-82ae47cc2965",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(name = "Necroblossom Snarl", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no EnterModifier variant says \"reveal a Swamp or Forest card from your \
         hand\" — TappedUnless and its siblings ask about a permanent on the \
         battlefield, and the choice is made in hand"
    ),
    // NOT SUPPORTED: "As this land enters, you may reveal a Swamp or Forest
    // card from your hand. If you don't, this land enters tapped."
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Black,
        ManaColor::Green,
    ])])],
);
