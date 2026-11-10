//! Fighting Chance — {R} — Instant
//! Oracle: For each blocking creature, flip a coin. If you win the flip, prevent all combat damage that would be dealt by that creature this turn.
//! Set: EXO #82 — Exodus | Scryfall ID: ca75f6e9-5eee-4904-88c0-71ec730a0f23 | Oracle ID: fece632b-d8d5-4d3d-a4a7-643293b67538
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

static BLOCKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Blocking]);

card!(
    index = index::FIGHTING_CHANCE,
    oracle_id = "fece632b-d8d5-4d3d-a4a7-643293b67538",
    scryfall_id = "ca75f6e9-5eee-4904-88c0-71ec730a0f23",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fighting Chance",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ForEachMatching {
        filter: &BLOCKING_CREATURE,
        effects: &[Effect::FlipCoin {
            won: &[Effect::ModifyEvent {
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            }],
            lost: &[],
        }],
    }])],
);
