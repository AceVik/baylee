//! Maze of Ith — (no cost) — Land
//! Oracle: {T}: Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Set: DMR #250 — Dominaria Remastered | Scryfall ID: 5889fde1-730d-43d0-aaa4-499784a80530 | Oracle ID: 38a12bd7-4394-44a8-91a0-6a4ff7fa4f71
// IMPLEMENTED — untap + damage prevention to/from the target until EOT.

static ATTACKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Attacking]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 93,
    oracle_id: "38a12bd7-4394-44a8-91a0-6a4ff7fa4f71",
    scryfall_id: "5889fde1-730d-43d0-aaa4-499784a80530",
    faces: &[face! {
        name: "Maze of Ith",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[activated!(Cost::TAP, &[
            Effect::UntapTarget,
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
        ], target: Some(TargetSpec::Object(&ATTACKING_CREATURE)))],
}
