//! Kazuul's Fury // Kazuul's Cliffs — {2}{R} — Instant // Land
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Kazuul's Fury deals damage equal to the sacrificed creature's power to any target.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #146 — Zendikar Rising | Scryfall ID: 75240bbc-adc7-48ff-9523-c79776d710d3 | Oracle ID: f8410804-632b-4f18-9a73-6dccc7e4582d
//! Face: Kazuul's Fury — {2}{R} — Instant
//! Face: Kazuul's Cliffs —  — Land
// IMPLEMENTED — Kazuul's Fury sacrifices a creature as an additional cost
// and deals damage equal to its power (Amount::SacrificedPower) to any
// target; Kazuul's Cliffs enters tapped and taps for {R}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KAZUUL_S_FURY,
    oracle_id = "f8410804-632b-4f18-9a73-6dccc7e4582d",
    scryfall_id = "75240bbc-adc7-48ff-9523-c79776d710d3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Kazuul's Fury",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
            mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
            abilities = &[spell!(
                &[Effect::DealDamage {
                    amount: Amount::SacrificedPower,
                    target: TargetSpec::AnyTarget,
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyTarget))
            )],
        ),
        face!(
            name = "Kazuul's Cliffs",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
);
