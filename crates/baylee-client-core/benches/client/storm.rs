//! A target for each Ally, rather than thousands of copies of one target.

use baylee_client_core::board::{BoardModel, Openings, Registry};
use baylee_client_core::test_support::{ViewBuilder, target, token};
use criterion::Criterion;
use std::hint::black_box;

pub fn bench(c: &mut Criterion) {
    for count in [100, 1_000, 3_000] {
        let allies: Vec<_> = (0..count).map(|id| token(id, 0, "Ally", 1, 1)).collect();
        let source = token(count, 0, "Rally source", 4, 4);
        let stack = allies
            .iter()
            .map(|ally| {
                let mut ability = token(count + 1 + ally.id.slot(), 0, "Rally", 0, 0);
                ability.stack_item = Some(baylee_view::StackItem::Ability {
                    source: source.id,
                    ability: None,
                    text: None,
                    rules: None,
                    token: None,
                });
                ability.targets.push(target(ally.id));
                ability
            })
            .collect();
        let view = ViewBuilder::new(2)
            .with_battlefield(0, allies.into_iter().chain([source]))
            .with_stack(stack)
            .build();
        let registry = Registry {
            named: &|_| None,
            token_name: &|_| None,
            token_face: &|_| None,
            card_face: None,
            utility_land: &|_| false,
            printed_body: &|_| None,
        };
        c.bench_function(&format!("storm/board/{count}"), |b| {
            b.iter(|| {
                black_box(BoardModel::from_view(
                    black_box(&view),
                    Openings::none(),
                    &[],
                    registry,
                ))
            });
        });
    }
}
