//! The accumulated per-Ally haste effects left by a resolved rally storm.

use baylee_cards_dsl::{Duration, KeywordSet, Layer, Modifier};
use baylee_core::ids::{EffectId, PlayerId};
use baylee_engine::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_engine::zone::ZoneLocation;
use criterion::Criterion;
use std::hint::black_box;

pub fn bench(c: &mut Criterion) {
    for count in [100, 1_000, 3_000] {
        let mut state = super::token_board(count);
        for id in state.zones.list(ZoneLocation::Battlefield).clone() {
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: PlayerId::new(0),
                origin: EffectOrigin::Resolution,
                layer: Layer::Ability,
                timestamp: 1,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::object(&state, id),
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            });
        }
        c.bench_function(&format!("rally/refresh/{count}"), |b| {
            b.iter(|| {
                state.invalidate_projections();
                state.refresh_characteristics();
                black_box(&state);
            });
        });
    }
}
