//! Browse — {2}{U}{U} — Enchantment
//! Oracle: {2}{U}{U}: Look at the top five cards of your library, put one of them into your hand, and exile the rest.
//! Set: ME2 #43 — Masters Edition II | Scryfall ID: 6c3dc360-4f7b-49bc-801a-89eb06fbb729 | Oracle ID: 49491601-e272-4603-9348-185a2953bd4b
// NOT SUPPORTED: exiling the remaining four cards looked at is not supported; LookAtTopPick puts them on the bottom of your library instead.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BROWSE,
    oracle_id = "49491601-e272-4603-9348-185a2953bd4b",
    scryfall_id = "6c3dc360-4f7b-49bc-801a-89eb06fbb729",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "{2}{U}{U} looks at the top five cards of your library and puts one of them into your hand; puts the rest on the bottom of your library instead of exiling them",
    ),
    faces = &[face!(
        name = "Browse",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{U}{U}"),
        &[Effect::LookAtTopPick {
            count: Amount::Fixed(5),
            pick: 1,
            random: false,
        }],
    )],
);
