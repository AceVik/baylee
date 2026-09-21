//! Kazuul's Fury // Kazuul's Cliffs — {2}{R} — Instant // Land
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Kazuul's Fury deals damage equal to the sacrificed creature's power to any target.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #146 — Zendikar Rising | Scryfall ID: 75240bbc-adc7-48ff-9523-c79776d710d3 | Oracle ID: f8410804-632b-4f18-9a73-6dccc7e4582d
//! Face: Kazuul's Fury — {2}{R} — Instant
//! Face: Kazuul's Cliffs —  — Land
// PARTIAL — Kazuul's Cliffs is built in full: it enters tapped and taps for
// {R}. Kazuul's Fury is not built at all — the face carries no ability, so
// casting it would resolve and do nothing.

// NOT SUPPORTED: "As an additional cost to cast this spell, sacrifice a
// creature." No spell cost list pays CostPart::Sacrifice: additional_costs
// reads only its mana and mandatory_additional_costs pays only PayLifeX and
// PayLife, and a part written on a list whose payment walks past it is refused
// by offer_tests::no_spell_cost_list_carries_a_part_its_payment_walks_past.
// NOT SUPPORTED: "Kazuul's Fury deals damage equal to the sacrificed
// creature's power to any target." No Amount reads the power of a creature
// sacrificed to pay a cost — Amount::TargetPower is the first *target*'s
// power, and the sacrificed creature is never a target.

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
        ),
        face!(
            name = "Kazuul's Cliffs",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
        ),
    ],
    coverage = Coverage::Partial(
        "Kazuul's Fury: the sacrifice-as-additional-cost is paid by no spell \
         cost list, and no Amount reads the sacrificed creature's power"
    ),
);
