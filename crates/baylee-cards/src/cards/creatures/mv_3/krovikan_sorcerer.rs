//! Krovikan Sorcerer — {2}{U} — Creature — Human Wizard Sorcerer
//! Oracle: {T}, Discard a nonblack card: Draw a card.
//! Oracle: {T}, Discard a black card: Draw two cards, then discard one of them.
//! Set: VMA #77 — Vintage Masters | Scryfall ID: 62909d11-9d2f-42a7-9b73-6b494bbe3f9e | Oracle ID: 00e6808c-1f9e-4b6a-807c-83f433b81a41
// NOT SUPPORTED: "{T}, Discard a black card: Draw two cards, then discard one of them." No effect discards one of the cards drawn this way, and discarding any card from hand would let the player throw away a card they already held, so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONBLACK_CARD: Filter =
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])));

card!(
    index = index::KROVIKAN_SORCERER,
    oracle_id = "00e6808c-1f9e-4b6a-807c-83f433b81a41",
    scryfall_id = "62909d11-9d2f-42a7-9b73-6b494bbe3f9e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the second ability ({T}, Discard a black card: Draw two cards, then discard one of them) is missing: no effect restricts the discard to the cards drawn this way",
    ),
    faces = &[face!(
        name = "Krovikan Sorcerer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::SORCERER,
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Discard(&NONBLACK_CARD)),
        &[Effect::draw(1)],
    )],
);
