//! Worthy Cause — {W} — Instant
//! Oracle: Buyback {2} (You may pay an additional {2} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: You gain life equal to the sacrificed creature's toughness.
//! Set: TMP #53 — Tempest | Scryfall ID: 1f610d8b-1782-43a4-bfb3-40887bdedba0 | Oracle ID: b24063f7-157f-47c0-919a-33856d53b902
// PARTIAL — Sacrifices a creature as an additional cost and gains life equal to the sacrificed creature's toughness; Buyback {2} (paying an additional {2} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORTHY_CAUSE,
    oracle_id = "b24063f7-157f-47c0-919a-33856d53b902",
    scryfall_id = "1f610d8b-1782-43a4-bfb3-40887bdedba0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Sacrifices a creature as an additional cost and gains life equal to the sacrificed creature's toughness; Buyback {2} (paying an additional {2} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Worthy Cause",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::SacrificedToughness,
    }])],
);
