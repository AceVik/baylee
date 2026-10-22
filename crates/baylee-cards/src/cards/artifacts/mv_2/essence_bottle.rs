//! Essence Bottle — {2} — Artifact
//! Oracle: {3}, {T}: Put an elixir counter on this artifact.
//! Oracle: {T}, Remove all elixir counters from this artifact: You gain 2 life for each elixir counter removed this way.
//! Set: TMP #286 — Tempest | Scryfall ID: 7e3c7760-04d8-424d-a097-df0ca8297837 | Oracle ID: 76f839b0-1d4c-4ad9-b44f-a8a13d6d109f
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ESSENCE_BOTTLE,
    oracle_id = "76f839b0-1d4c-4ad9-b44f-a8a13d6d109f",
    scryfall_id = "7e3c7760-04d8-424d-a097-df0ca8297837",
    coverage = Coverage::Partial(
        "Puts an elixir counter on itself for {3}, {T}, and for {T} gains 2 life for each elixir counter on it and removes all elixir counters from it on resolution (rather than removing all elixir counters as an activation cost)."
    ),
    faces = &[face!(
        name = "Essence Bottle",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::AddCounterSource {
                kind: counters::ELIXIR,
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(TapSelf),
            &[
                Effect::GainLife {
                    amount: Amount::Times {
                        base: &Amount::CountersOnSource(counters::ELIXIR),
                        factor: 2,
                    },
                },
                Effect::RemoveAllCountersSelf {
                    kind: counters::ELIXIR,
                },
            ],
        ),
    ],
);
