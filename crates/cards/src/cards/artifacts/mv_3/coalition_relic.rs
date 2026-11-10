//! Coalition Relic — {3} — Artifact
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {T}: Put a charge counter on this artifact.
//! Oracle: At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way.
//! Set: DMC #180 — Dominaria United Commander | Scryfall ID: ef44324a-32bd-47e9-8fd9-258ba668de53 | Oracle ID: 008cb342-79f5-4df6-a6b7-0e9e22ed693f
// IMPLEMENTED — immediate mana, charge storage, and first-main mana release.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COALITION_RELIC,
    oracle_id = "008cb342-79f5-4df6-a6b7-0e9e22ed693f",
    scryfall_id = "ef44324a-32bd-47e9-8fd9-258ba668de53",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Coalition Relic",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(ALL_MANA_COLORS)]),
        activated!(
            Cost::TAP,
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::FirstMain,
                whose: PlayerRel::You,
            },
            &[
                Effect::RemoveAllCountersSelf {
                    kind: CounterKind::Charge,
                },
                Effect::mana_combination(
                    ALL_MANA_COLORS,
                    Amount::ThisWay(ThisWay::CountersRemoved(CounterKind::Charge)),
                ),
            ],
        ),
    ],
);
