//! Nantuko Monastery — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Threshold — {G}{W}: This land becomes a 4/4 green and white Insect Monk creature with first strike until end of turn. It's still a land. Activate only if there are seven or more cards in your graveyard.
//! Set: DMR #252 — Dominaria Remastered | Scryfall ID: 4188a561-46e3-4ddd-8797-f33c37e9adb2 | Oracle ID: c6de0ee9-785d-4cd8-8a7f-5bb715763131
// IMPLEMENTED — {T}: Add {C}. The threshold animate ability is dropped (see
// NOT SUPPORTED below): the animation is sayable, its printed activation
// gate is not, and an ungated version is a land that attacks for four.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NANTUKO_MONASTERY,
    oracle_id = "c6de0ee9-785d-4cd8-8a7f-5bb715763131",
    scryfall_id = "4188a561-46e3-4ddd-8797-f33c37e9adb2",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Nantuko Monastery", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "threshold gate on the animate ability: no Condition for seven or more cards in your graveyard"
    ),
    // NOT SUPPORTED: Threshold — {G}{W}: This land becomes a 4/4 green and
    // white Insect Monk creature with first strike until end of turn. It's
    // still a land. Activate only if there are seven or more cards in your
    // graveyard. — the animation is expressible (AddType(CREATURE),
    // AddSubtype(INSECT)/(MONK), AddColor, SetPT(4, 4) and
    // AddKeyword(FIRST_STRIKE), each on Filter::This until end of turn), but
    // activating it only at threshold is not: `Condition` states your
    // battlefield count (ControlCount), an opponent's graveyard
    // (OpponentGraveyardCountAtLeast), counters on the source
    // (CountersOnSelf) or the source's own filter (SourceMatches), and none
    // of those is "seven or more cards in your graveyard". So the ability
    // comes off the card rather than animating a land that never reached
    // threshold.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
