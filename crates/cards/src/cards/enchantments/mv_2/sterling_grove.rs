//! Sterling Grove — {G}{W} — Enchantment
//! Oracle: Other enchantments you control have shroud. (They can't be the targets of spells or abilities.)
//! Oracle: {1}, Sacrifice this enchantment: Search your library for an enchantment card, reveal it, then shuffle and put that card on top.
//! Set: MH2 #293 — Modern Horizons 2 | Scryfall ID: ba03e105-a76c-4769-a35a-d780448890ec | Oracle ID: 2c275a85-5a15-46cd-a6e7-add63f9b853d
// IMPLEMENTED — a static layer-6 grant of shroud to your other enchantments,
// and {1}, sacrifice this: search your library for an enchantment card onto
// the top of your library (the reveal and the shuffle are derived).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STERLING_GROVE,
    oracle_id = "2c275a85-5a15-46cd-a6e7-add63f9b853d",
    scryfall_id = "ba03e105-a76c-4769-a35a-d780448890ec",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Sterling Grove",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(
            f!(your another ENCHANTMENT),
            Modifier::AddKeyword(KeywordSet::SHROUD)
        ),
        activated!(
            cost!("{1}", SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::ENCHANTMENT,
                finds: &[Find::TOP_OF_LIBRARY],
                optional: false,
            }]
        ),
    ],
);
