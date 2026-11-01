//! Foreshadow — {1}{U} — Instant
//! Oracle: Choose a card name, then target opponent mills a card. If a card with the chosen name was milled this way, you draw a card.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: VIS #33 — Visions | Scryfall ID: d54c51de-bfac-4198-a7c4-37b4db74e525 | Oracle ID: 4caa97df-1f6c-4806-bf14-e589cc1badb5
// PARTIAL — Checking whether the card milled by target opponent has the chosen name so you draw a card is not yet supported in Effect; in play, Foreshadow chooses a card name, causes target opponent to mill a card, and draws a card at the beginning of the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORESHADOW,
    oracle_id = "4caa97df-1f6c-4806-bf14-e589cc1badb5",
    scryfall_id = "d54c51de-bfac-4198-a7c4-37b4db74e525",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Checking whether the card milled by target opponent has the chosen name so you draw a card is not yet supported in Effect; in play, Foreshadow chooses a card name, causes target opponent to mill a card, and draws a card at the beginning of the next turn's upkeep.",
    ),
    faces = &[face!(
        name = "Foreshadow",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ChooseCardName {
                player: PlayerRel::You,
                types: TypeSet::EMPTY,
                excluded_types: TypeSet::EMPTY,
                remember: false,
            },
            Effect::Mill {
                amount: Amount::Fixed(1),
                target: PlayerRel::Chosen,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
