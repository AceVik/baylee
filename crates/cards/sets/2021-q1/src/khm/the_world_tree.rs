//! The World Tree — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: As long as you control six or more lands, lands you control have "{T}: Add one mana of any color."
//! Oracle: {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land: Search your library for any number of God cards, put them onto the battlefield, then shuffle.
//! Set: KHM #275 — Kaldheim | Scryfall ID: a70cb6d9-3955-4064-917b-11dec26440c5 | Oracle ID: 3437d504-bf62-4c27-b15f-f6330182ff7e
// IMPLEMENTED — enters tapped and taps for {G}; while its controller has six
// or more lands, every land they control has "{T}: Add one mana of any
// color" (a conditional GrantActivated static); and the ten-mana sacrifice
// searches for any number of God cards (one repeating find) and puts them
// onto the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_WORLD_TREE,
    oracle_id = "3437d504-bf62-4c27-b15f-f6330182ff7e",
    scryfall_id = "a70cb6d9-3955-4064-917b-11dec26440c5",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "The World Tree",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // "As long as you control six or more lands, lands you control have
        // '{T}: Add one mana of any color.'"
        static_ability!(
            Filter::YOUR_LAND,
            Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: ANY_COLOR_MANA,
                mana_ability: true,
            },
            condition = Some(Condition::ControlCount(&Filter::YOUR_LAND, 6))
        ),
        // "{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land: Search
        // your library for any number of God cards, put them onto the
        // battlefield, then shuffle."
        activated!(
            cost!("{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}", TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::HasSubtype(subtypes::creature::GOD),
                finds: &[Find::BATTLEFIELD.any_number()],
                optional: true,
            }]
        ),
    ],
);
