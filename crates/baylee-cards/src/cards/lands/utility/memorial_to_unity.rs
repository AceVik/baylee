//! Memorial to Unity — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}, Sacrifice this land: Look at the top five cards of your library. You may reveal a creature card from among them and put it into your hand. Then put the rest on the bottom of your library in a random order.
//! Set: DOM #245 — Dominaria | Scryfall ID: 994d903b-0d54-4af4-898f-213e716e9ed4 | Oracle ID: a74494ef-aa35-4830-9b4c-47bff5270efc
// PARTIAL — enters tapped and taps for {G}; the Memorial's dig clause has no
// DSL spelling, so that ability is off the card (see the line at the foot).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORIAL_TO_UNITY,
    oracle_id = "a74494ef-aa35-4830-9b4c-47bff5270efc",
    scryfall_id = "994d903b-0d54-4af4-898f-213e716e9ed4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no variant for \"look at the top five, reveal a creature card into hand, \
         rest on the bottom in a random order\": Effect::LookAtTopPick takes no \
         filter, offers no \"you may\", and bottoms the rest in any order",
    ),
    faces = &[face!(
        name = "Memorial to Unity",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);

// NOT SUPPORTED: {2}{G}, {T}, Sacrifice this land: Look at the top five cards of
// your library. You may reveal a creature card from among them and put it into
// your hand. Then put the rest on the bottom of your library in a random order.
// Effect::LookAtTopPick { count, pick } is the nearest shape and is a different
// card three times over: its pick is unfiltered (any card, not "a creature
// card"), it does not ask the printed "you may", and it puts the remainder back
// in any order where the card says "in a random order".
