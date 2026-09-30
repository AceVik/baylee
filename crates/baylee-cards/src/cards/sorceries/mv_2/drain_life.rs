//! Drain Life — {X}{1}{B} — Sorcery
//! Oracle: Spend only black mana on X.
//! Oracle: Drain Life deals X damage to any target. You gain life equal to the damage dealt, but not more life than the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's toughness.
//! Set: 5ED #156 — Fifth Edition | Scryfall ID: 41a9b3b2-4ac9-4e50-bcd2-8831d0739e85 | Oracle ID: e75ba79f-4cc2-4ede-8641-559ab94e7e36
// PARTIAL — black mana only for X and the capped life gain are not in the
// engine; it deals X damage.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAIN_LIFE,
    oracle_id = "e75ba79f-4cc2-4ede-8641-559ab94e7e36",
    scryfall_id = "41a9b3b2-4ac9-4e50-bcd2-8831d0739e85",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "black mana only for X and the capped life gain are not in the engine; it deals X damage"
    ),
    faces = &[face!(
        name = "Drain Life",
        mana_cost = mana!("{X}{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::X,
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
        // NOT SUPPORTED: Spend only black mana on X.
        // NOT SUPPORTED: You gain life equal to the damage dealt, but not more life than
        // the player's life total before the damage was dealt, the planeswalker's loyalty
        // before the damage was dealt, or the creature's toughness.
    ],
);
