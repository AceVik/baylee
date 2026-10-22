//! Lobotomy — {2}{U}{B} — Sorcery
//! Oracle: Target player reveals their hand, then you choose a card other than a basic land card from it. Search that player's graveyard, hand, and library for all cards with the same name as the chosen card and exile them. Then that player shuffles.
//! Set: INV #255 — Invasion | Scryfall ID: ff307dbb-4ab6-457b-be56-47106864bf61 | Oracle ID: a7ebe748-c3de-402f-8a6e-236a6413c0f8
// PARTIAL: target player reveals their hand and discards a non-basic-land card of your choice; exiling that card and searching their graveyard, hand, and library for all cards with the same name to exile is not in Effect.

use baylee_cards_dsl::prelude::*;

static NON_BASIC_LAND: Filter = Filter::Not(&Filter::BASIC_LAND);

card!(
    index = index::LOBOTOMY,
    oracle_id = "a7ebe748-c3de-402f-8a6e-236a6413c0f8",
    scryfall_id = "ff307dbb-4ab6-457b-be56-47106864bf61",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "target player reveals their hand and discards a non-basic-land card of your choice; exiling that card and searching their graveyard, hand, and library for all cards with the same name to exile is not in Effect",
    ),
    faces = &[face!(
        name = "Lobotomy",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &NON_BASIC_LAND,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
