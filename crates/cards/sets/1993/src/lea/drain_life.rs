//! Drain Life — {X}{1}{B} — Sorcery
//! Oracle: Spend only black mana on X.
//! Oracle: Drain Life deals X damage to any target. You gain life equal to the damage dealt, but not more life than the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's toughness.
//! Set: 5ED #156 — Fifth Edition | Scryfall ID: 41a9b3b2-4ac9-4e50-bcd2-8831d0739e85 | Oracle ID: e75ba79f-4cc2-4ede-8641-559ab94e7e36

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAIN_LIFE,
    oracle_id = "e75ba79f-4cc2-4ede-8641-559ab94e7e36",
    scryfall_id = "41a9b3b2-4ac9-4e50-bcd2-8831d0739e85",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Drain Life",
        mana_cost = mana!("{X}{1}{B}"),
        x_mana_color = Some(Color::Black),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageWithCappedLifeGain { amount: Amount::X }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    ),],
);
