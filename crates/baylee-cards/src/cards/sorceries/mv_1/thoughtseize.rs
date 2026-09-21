//! Thoughtseize — {B} — Sorcery
//! Oracle: Target player reveals their hand. You choose a nonland card from it. That player discards that card. You lose 2 life.
//! Set: 2XM #109 — Double Masters | Scryfall ID: b281a308-ab6b-47b6-bec7-632c9aaecede | Oracle ID: edd8d1e8-be43-4c38-bb3a-83081fbaf0b5
// PARTIAL — "You lose 2 life" (with the printed player target) is built; the
// reveal-and-discard clause has no DSL vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOUGHTSEIZE,
    oracle_id = "edd8d1e8-be43-4c38-bb3a-83081fbaf0b5",
    scryfall_id = "b281a308-ab6b-47b6-bec7-632c9aaecede",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Thoughtseize",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no Effect reveals a target player's hand, and no Effect lets the \
         caster choose a card from another player's hand for that player to \
         discard (DiscardForPlayers has the affected player choose)",
    ),
    abilities = &[spell!(
        &[
            // NOT SUPPORTED: "Target player reveals their hand. You choose a
            // nonland card from it. That player discards that card." — the
            // hand is never revealed by any Effect, and the discard family
            // (`DiscardForPlayers`) is the affected player's own choice,
            // which is a different card.
            Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
