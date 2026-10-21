//! Fire Covenant — {1}{B}{R} — Instant
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: Fire Covenant deals X damage divided as you choose among any number of target creatures.
//! Set: ECC #54 — Lorwyn Eclipsed Commander | Scryfall ID: 1a13437b-e0c7-4ca9-a0e2-b8cb1b5813ec | Oracle ID: 025939a0-424a-41bf-8fc9-2ef9ea5485f7
// PARTIAL: X is always 1 — the card plays as if cast with X = 1.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: X. `CostPart::PayLifeX` pays an announced X as life (Toxic
// Deluge), but `DivisionAmount` divides only a printed number or Shatterskull
// Smashing's doubled X, so "X damage divided as you choose" has no total.
// Until a plain X total exists the card is what it is with X = 1: pay 1 life,
// 1 damage to up to one target creature (each target needs at least 1,
// CR 601.2d).
card!(
    index = index::FIRE_COVENANT,
    oracle_id = "025939a0-424a-41bf-8fc9-2ef9ea5485f7",
    scryfall_id = "1a13437b-e0c7-4ca9-a0e2-b8cb1b5813ec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "X is fixed at 1 (pay 1 life, 1 damage to up to one target creature): no DivisionAmount divides an announced X"
    ),
    faces = &[face!(
        name = "Fire Covenant",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLife(1)],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
