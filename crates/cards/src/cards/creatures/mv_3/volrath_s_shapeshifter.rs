//! Volrath's Shapeshifter — {1}{U}{U} — Creature — Phyrexian Shapeshifter
//! Oracle: As long as the top card of your graveyard is a creature card, this creature has the full text of that card and has the text "{2}: Discard a card." (This creature has that card's name, mana cost, color, types, abilities, power, and toughness.)
//! Oracle: {2}: Discard a card.
//! Set: VMA #101 — Vintage Masters | Scryfall ID: 93ed24e6-7791-4624-acf2-aa8616035f9e | Oracle ID: 16e9a4ee-0398-4e76-bc7e-dd2d52056aaf
// PARTIAL: taking the full text and characteristics of the top creature card of your graveyard is not supported; implements the "{2}: Discard a card" activated ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOLRATH_S_SHAPESHIFTER,
    oracle_id = "16e9a4ee-0398-4e76-bc7e-dd2d52056aaf",
    scryfall_id = "93ed24e6-7791-4624-acf2-aa8616035f9e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "taking the full text and characteristics of the top creature card of your graveyard is not supported; implements the \"{2}: Discard a card\" activated ability",
    ),
    faces = &[face!(
        name = "Volrath's Shapeshifter",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::SHAPESHIFTER
        ],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[Effect::DiscardCount {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        }],
    )],
);
