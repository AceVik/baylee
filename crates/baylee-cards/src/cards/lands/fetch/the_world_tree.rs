//! The World Tree — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: As long as you control six or more lands, lands you control have "{T}: Add one mana of any color."
//! Oracle: {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land: Search your library for any number of God cards, put them onto the battlefield, then shuffle.
//! Set: KHM #275 — Kaldheim | Scryfall ID: a70cb6d9-3955-4064-917b-11dec26440c5 | Oracle ID: 3437d504-bf62-4c27-b15f-f6330182ff7e
// PARTIAL — enters tapped, and it taps for {G}; the conditional grant and the
// unbounded God search stay unimplemented.

use baylee_cards_dsl::prelude::*;

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
    coverage = Coverage::Partial(
        "the land grant is conditional on a land count and a StaticAbility carries no \
         Condition, and \"any number of God cards\" is a search count SearchLibrary's \
         fixed `finds` slice cannot carry"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);

// NOT SUPPORTED: As long as you control six or more lands, lands you control
// have "{T}: Add one mana of any color." — Modifier::GrantActivated grants
// unconditionally; nothing conditions a static ability.
// NOT SUPPORTED: {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land:
// Search your library for any number of God cards, put them onto the
// battlefield, then shuffle. — Effect::SearchLibrary finds `finds.len()`
// cards at most, and "any number" is unbounded.
