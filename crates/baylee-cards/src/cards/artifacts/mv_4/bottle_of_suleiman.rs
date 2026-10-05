//! Bottle of Suleiman — {4} — Artifact
//! Oracle: {1}, Sacrifice this artifact: Flip a coin. If you win the flip, create a 5/5 colorless Djinn artifact creature token with flying. If you lose the flip, this artifact deals 5 damage to you.
//! Set: ME4 #184 — Masters Edition IV | Scryfall ID: ab5afd8a-f689-4f5a-9c60-96f09c92e5a0 | Oracle ID: f32d19d6-8ac1-4744-b2c2-5c9d4cd0da70
// PARTIAL — the coin-flip ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOTTLE_OF_SULEIMAN,
    oracle_id = "f32d19d6-8ac1-4744-b2c2-5c9d4cd0da70",
    scryfall_id = "ab5afd8a-f689-4f5a-9c60-96f09c92e5a0",
    faces = &[face!(
        name = "Bottle of Suleiman",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the ability's whole resolution is a coin flip and the DSL has no coin-flip \
         effect, so the win and lose branches have nothing that chooses between them",
    ),
    // NOT SUPPORTED: "{1}, Sacrifice this artifact: Flip a coin. If you win the
    // flip, create a 5/5 colorless Djinn artifact creature token with flying.
    // If you lose the flip, this artifact deals 5 damage to you." — both
    // branches are ordinary effects (Effect::CreateToken, Effect::DealDamage
    // to PlayerRel::You) with the sacrifice already a cost, but no Effect or
    // ReplacementRule flips a coin (the engine's GameRng::below(2) has no
    // vocabulary), so the ability comes off the card rather than resolving
    // one branch for free.
);
