//! Differential checks against the complete plan, including stale incarnations.

use super::*;

#[test]
fn indexed_projection_matches_complete_plan_for_mixed_effects() {
    let mut random = 0xace5_1234_u64;
    for case in 0..80 {
        let mut state = fresh();
        let ids: Vec<_> = (0..12)
            .map(|n| {
                permanent(
                    &mut state,
                    "Body",
                    if n % 3 == 0 {
                        TypeSet::ARTIFACT
                    } else {
                        TypeSet::CREATURE
                    },
                    Some((2, 3)),
                )
            })
            .collect();
        for n in 0..(32 + case * 2) {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let id = ids[(random >> 32) as usize % ids.len()];
            let modifier = match n % 9 {
                0 => Modifier::AddKeyword(KeywordSet::HASTE),
                1 => Modifier::LoseKeywords,
                2 => Modifier::ModifyPT(1, -1),
                3 => Modifier::AddType(TypeSet::ARTIFACT),
                4 => Modifier::RemoveType(TypeSet::CREATURE),
                5 => Modifier::AnimateNoncreatureArtifact,
                6 => Modifier::AddColor(ColorSet::of(Color::Red)),
                7 => Modifier::AddKeyword(KeywordSet::FLYING),
                _ => Modifier::LoseAllAbilities,
            };
            let filter = match n % 5 {
                0 => EffectFilter::Dsl(&READS_TYPE),
                1 => EffectFilter::Dsl(&READS_KW),
                2 => EffectFilter::ObjectIs(id, state.object(id).unwrap().version + 1),
                _ => EffectFilter::object(&state, id),
            };
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: me(),
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp: random % 7,
                duration: Duration::UntilEndOfTurn,
                filter,
                modifier,
            });
        }
        let indexed = LayerPlan::build(&state.effects);
        assert!(indexed.index.is_some());
        let mut reference = indexed.clone();
        reference.index = None;
        for id in &ids {
            let object = state.object(*id).unwrap();
            let actual = recompute_with(&state, object, &indexed);
            let expected = recompute_with(&state, object, &reference);
            assert_eq!(
                actual.characteristics, expected.characteristics,
                "case {case}, {id:?}"
            );
            assert_eq!(actual.controller, expected.controller);
            assert_eq!(actual.read_board, expected.read_board);
        }
    }
}

#[test]
fn an_indexed_plan_keeps_only_its_objects_effects_in_the_original_order() {
    let mut state = fresh();
    let a = permanent(&mut state, "A", TypeSet::CREATURE, Some((1, 1)));
    let b = permanent(&mut state, "B", TypeSet::CREATURE, Some((1, 1)));
    for n in 0..1_000 {
        let mut effect = fx(0, n % 3, &ANY_F, Modifier::AddKeyword(KeywordSet::HASTE));
        effect.layer = Layer::Ability;
        effect.filter = EffectFilter::object(&state, if n % 2 == 0 { a } else { b });
        state.effects.register(effect);
    }
    let plan = LayerPlan::build(&state.effects);
    for id in [a, b] {
        let expected: Vec<_> = plan
            .layer(Layer::Ability)
            .iter()
            .copied()
            .filter(|&at| {
                state.effects.as_slice()[at as usize]
                    .filter
                    .names(state.object(id).unwrap())
            })
            .collect();
        assert_eq!(
            plan.candidates(Layer::Ability, id).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(expected.len(), 500);
    }
    assert!(!plan.may_affect(ObjectId::new(999_999, 0)));
}
