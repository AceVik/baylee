//! Molten Hydra — {1}{R} — Creature — Hydra
//! Oracle: {1}{R}{R}: Put a +1/+1 counter on this creature.
//! Oracle: {T}, Remove all +1/+1 counters from this creature: It deals damage to any target equal to the number of +1/+1 counters removed this way.
//! Set: ULG #85 — Urza's Legacy | Scryfall ID: 95234b29-9ac8-4200-b42d-9653ba51b010 | Oracle ID: cd5bc95d-4915-4c5a-9b57-2730adf69797
// PARTIAL — removing all +1/+1 counters as an activation cost is unsupported; damage ability omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOLTEN_HYDRA,
    oracle_id = "cd5bc95d-4915-4c5a-9b57-2730adf69797",
    scryfall_id = "95234b29-9ac8-4200-b42d-9653ba51b010",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Removing all +1/+1 counters as an activation cost is unsupported; the damage ability is omitted.",
    ),
    faces = &[face!(
        name = "Molten Hydra",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HYDRA],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{1}{R}{R}"),
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
        // NOT SUPPORTED: Remove all +1/+1 counters as a cost, then deal that much damage.
    ],
);
