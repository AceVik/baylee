//! Beacon of Destruction — {3}{R}{R} — Instant
//! Oracle: Beacon of Destruction deals 5 damage to any target. Shuffle Beacon of Destruction into its owner's library.
//! Set: DDS #35 — Duel Decks: Mind vs. Might | Scryfall ID: 1ece7e7d-615b-42da-8bab-80f24ad7c4d8 | Oracle ID: 15feb59b-bbbe-462a-8d38-d20735bbc2f3
// IMPLEMENTED — 5 damage to any target then shuffles self into owner's library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BEACON_OF_DESTRUCTION,
    oracle_id = "15feb59b-bbbe-462a-8d38-d20735bbc2f3",
    scryfall_id = "1ece7e7d-615b-42da-8bab-80f24ad7c4d8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Beacon of Destruction",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(5),
                target: TargetSpec::AnyTarget,
            },
            Effect::PutSourceOnTopOfLibrary,
            Effect::ShuffleLibrary {
                who: PlayerRel::OwnerOfSource,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
