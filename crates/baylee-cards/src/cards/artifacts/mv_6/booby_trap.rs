//! Booby Trap — {6} — Artifact
//! Oracle: As this artifact enters, choose an opponent and a card name other than a basic land card name.
//! Oracle: The chosen player reveals each card they draw.
//! Oracle: When the chosen player draws a card with the chosen name, sacrifice this artifact. If you do, this artifact deals 10 damage to that player.
//! Set: 9ED #289 — Ninth Edition | Scryfall ID: 1af0c9a0-0dfa-4245-8b29-7bd37982b7d2 | Oracle ID: 3e46381a-7850-4e50-9213-e1a180299977
// PARTIAL — As this artifact enters, you choose an opponent and a card name, and when an opponent draws a card this artifact is sacrificed and deals 10 damage to that player; restricting the chosen card name to non-basic-land names, revealing each card the chosen player draws, and triggering only when the chosen player draws a card with the chosen name are not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOOBY_TRAP,
    oracle_id = "3e46381a-7850-4e50-9213-e1a180299977",
    scryfall_id = "1af0c9a0-0dfa-4245-8b29-7bd37982b7d2",
    coverage = Coverage::Partial(
        "As this artifact enters, you choose an opponent and a card name, and when an opponent draws a card this artifact is sacrificed and deals 10 damage to that player; restricting the chosen card name to non-basic-land names, revealing each card the chosen player draws, and triggering only when the chosen player draws a card with the chosen name are not yet modeled."
    ),
    faces = &[face!(
        name = "Booby Trap",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseOpponent, EnterModifier::ChooseCardName],
    ),],
    abilities = &[triggered!(
        Trigger::Draws(PlayerRel::EachOpponent),
        &[Effect::SacrificeSelfThen {
            then: &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EventPlayer),
                amount: Amount::Fixed(10),
            }],
        }]
    )],
);
