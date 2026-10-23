//! Sonic Burst — {1}{R} — Instant
//! Oracle: As an additional cost to cast this spell, discard a card at random.
//! Oracle: Sonic Burst deals 4 damage to any target.
//! Set: EXO #103 — Exodus | Scryfall ID: 05530d5a-dcb6-403e-9e35-224c7b5cf615 | Oracle ID: 144d42b8-5c44-4a36-9028-75229047e56c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONIC_BURST,
    oracle_id = "144d42b8-5c44-4a36-9028-75229047e56c",
    scryfall_id = "05530d5a-dcb6-403e-9e35-224c7b5cf615",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Random discard as a mandatory additional casting cost is not supported in cast_wizard; deals 4 damage to any target without the additional discard cost",
    ),
    faces = &[face!(
        name = "Sonic Burst",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
        abilities = &[spell!(
            &[Effect::DealDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(4),
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        )],
    ),],
);
