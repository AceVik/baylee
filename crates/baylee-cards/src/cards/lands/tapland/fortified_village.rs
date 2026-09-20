//! Fortified Village — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {G} or {W}.
//! Set: MSC #245 — Marvel Super Heroes Commander | Scryfall ID: 15f81ac6-a550-4782-a3a9-22d7fef1c206 | Oracle ID: 56f1a16a-9f41-41fb-b580-c200bca27cd6
// PARTIAL — {T}: Add {G} or {W} is implemented; the reveal clause is the
// printed entry condition and has no EnterModifier variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORTIFIED_VILLAGE,
    oracle_id = "56f1a16a-9f41-41fb-b580-c200bca27cd6",
    scryfall_id = "15f81ac6-a550-4782-a3a9-22d7fef1c206",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    // NOT SUPPORTED: "As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped." — no EnterModifier asks about a card in hand.
    faces = &[face!(name = "Fortified Village", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the entry clause reveals a Forest or Plains card from hand, and no EnterModifier variant asks about a card in hand"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Green,
        ManaColor::White,
    ])])],
);
