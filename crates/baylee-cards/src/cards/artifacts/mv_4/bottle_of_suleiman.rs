//! Bottle of Suleiman — {4} — Artifact
//! Oracle: {1}, Sacrifice this artifact: Flip a coin. If you win the flip, create a 5/5 colorless Djinn artifact creature token with flying. If you lose the flip, this artifact deals 5 damage to you.
//! Set: ME4 #184 — Masters Edition IV | Scryfall ID: ab5afd8a-f689-4f5a-9c60-96f09c92e5a0 | Oracle ID: f32d19d6-8ac1-4744-b2c2-5c9d4cd0da70
// IMPLEMENTED — a sacrifice ability whose resolution is a coin flip
// (Effect::FlipCoin, CR 705): a 5/5 flying Djinn on a win, 5 damage to its
// controller on a loss.

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
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{1}", SacrificeSelf),
        &[Effect::FlipCoin {
            won: &[Effect::CreateToken {
                token: &crate::generated_tokens::DJINN_ARTIFACT_5_5_FLYING
            }],
            lost: &[Effect::DealDamage {
                amount: Amount::Fixed(5),
                target: TargetSpec::Player(PlayerRel::You),
            }],
        }]
    ),],
);
