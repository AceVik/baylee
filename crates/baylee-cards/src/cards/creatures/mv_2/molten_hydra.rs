//! Molten Hydra — {1}{R} — Creature — Hydra
//! Oracle: {1}{R}{R}: Put a +1/+1 counter on this creature.
//! Oracle: {T}, Remove all +1/+1 counters from this creature: It deals damage to any target equal to the number of +1/+1 counters removed this way.
//! Set: ULG #85 — Urza's Legacy | Scryfall ID: 95234b29-9ac8-4200-b42d-9653ba51b010 | Oracle ID: cd5bc95d-4915-4c5a-9b57-2730adf69797
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOLTEN_HYDRA,
    oracle_id = "cd5bc95d-4915-4c5a-9b57-2730adf69797",
    scryfall_id = "95234b29-9ac8-4200-b42d-9653ba51b010",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Has '{1}{R}{R}: Put a +1/+1 counter on this creature'; the second ability taps to deal damage equal to its +1/+1 counters to any target and removes all +1/+1 counters on resolution because removing all +1/+1 counters as an activation cost is not in CostPart.",
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
        activated!(
            Cost::TAP,
            &[
                Effect::DealDamage {
                    amount: Amount::CountersOnSource(CounterKind::P1P1),
                    target: TargetSpec::AnyTarget,
                },
                Effect::RemoveAllCountersSelf {
                    kind: CounterKind::P1P1,
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);
