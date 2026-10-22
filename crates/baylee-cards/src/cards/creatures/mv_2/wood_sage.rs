//! Wood Sage — {G}{U} — Creature — Human Druid
//! Oracle: {T}: Choose a creature card name. Reveal the top four cards of your library and put all of them with that name into your hand. Put the rest into your graveyard.
//! Set: TPR #216 — Tempest Remastered | Scryfall ID: baed8371-815d-4c46-aca7-1d704b7a4e52 | Oracle ID: 6fde7bb3-2539-411f-8733-36ce9595dfd5
// PARTIAL — Choosing a creature card name on resolution to put all matching cards from the top four of the library into hand is not in the DSL; mills four cards and lets you put up to one creature card from among them into your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WOOD_SAGE,
    oracle_id = "6fde7bb3-2539-411f-8733-36ce9595dfd5",
    scryfall_id = "baed8371-815d-4c46-aca7-1d704b7a4e52",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Partial(
        "choosing a creature card name on resolution to put all matching cards from the top four of the library into hand is not in the DSL; mills four cards and lets you put up to one creature card from among them into your hand"
    ),
    faces = &[face!(
        name = "Wood Sage",
        mana_cost = mana!("{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::MillMayTakeOne {
            amount: 4,
            filter: &Filter::CREATURE,
        }],
    )],
);
