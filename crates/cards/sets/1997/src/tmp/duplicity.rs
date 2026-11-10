//! Duplicity — {3}{U}{U} — Enchantment
//! Oracle: When this enchantment enters, exile the top five cards of your library face down.
//! Oracle: At the beginning of your upkeep, you may exile all cards from your hand face down. If you do, put all other cards you own exiled with this enchantment into your hand.
//! Oracle: At the beginning of your end step, discard a card.
//! Oracle: When you lose control of this enchantment, put all cards exiled with this enchantment into their owner's graveyard.
//! Set: TMP #60 — Tempest | Scryfall ID: d529cb33-292d-40e3-8cfe-db5eeb0d711e | Oracle ID: 01072a45-ab1c-4d19-b493-76cf4aae351e
// PARTIAL — Exiles the top five cards of your library face down when it enters and makes you discard a card at the beginning of your end step; the upkeep hand/linked-exile swap trigger and the control-loss trigger putting exiled cards into their owner's graveyard are not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUPLICITY,
    oracle_id = "01072a45-ab1c-4d19-b493-76cf4aae351e",
    scryfall_id = "d529cb33-292d-40e3-8cfe-db5eeb0d711e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Exiles the top five cards of your library face down when it enters and makes you discard a card at the beginning of your end step; the upkeep hand/linked-exile swap trigger and the control-loss trigger putting exiled cards into their owner's graveyard are not yet modeled."
    ),
    faces = &[face!(
        name = "Duplicity",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::ExileTopFaceDownLinked,
                Effect::ExileTopFaceDownLinked,
                Effect::ExileTopFaceDownLinked,
                Effect::ExileTopFaceDownLinked,
                Effect::ExileTopFaceDownLinked,
            ],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 1,
            }],
        ),
    ],
);
