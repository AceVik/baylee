//! `cards/creatures/mv_3/thelonite_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thelonite_druid() -> CardIndex {
    card_index("adbc8970-1bbc-4f30-9a2e-eebaf074eae6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Thelonite Druid (adbc8970-1bbc-4f30-9a2e-eebaf074eae6) — "{1}{G}, {T},
/// Sacrifice a creature: Forests you control become 2/3 creatures until end
/// of turn. They're still lands."
#[test]
fn thelonite_druid_animates_only_your_forests_and_the_effect_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                thelonite_druid(),
                forest(),
                forest(),
                island(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let druid = on_battlefield(&engine, p0, thelonite_druid()).expect("the Druid is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the sacrifice is out");
    let my_forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(my_forests.len(), 2);
    let my_island = on_battlefield(&engine, p0, island()).expect("an Island is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    for land in my_forests.iter().chain([&my_island, &their_forest]) {
        assert!(
            !types(&engine, *land).contains(TypeSet::CREATURE),
            "no land is a creature before the ability"
        );
    }

    float(
        &mut engine,
        0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    activate(&mut engine, p0, thelonite_druid(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&bears), "the Bears can be sacrificed");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("the Bears are on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the sacrificed creature is in the graveyard"
    );
    assert!(is_tapped(&engine, druid), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} paid"
    );
    for forest_id in &my_forests {
        let kinds = types(&engine, *forest_id);
        assert!(
            kinds.contains(TypeSet::CREATURE),
            "a Forest you control is a creature"
        );
        assert!(kinds.contains(TypeSet::LAND), "and still a land");
        assert_eq!(pt(&engine, *forest_id), (2, 3));
    }
    assert!(
        !types(&engine, my_island).contains(TypeSet::CREATURE),
        "an Island is not a Forest"
    );
    assert!(
        !types(&engine, their_forest).contains(TypeSet::CREATURE),
        "an opponent's Forest is not yours"
    );

    reach_their_main_phase(&mut engine, p1);
    for forest_id in &my_forests {
        assert!(
            !types(&engine, *forest_id).contains(TypeSet::CREATURE),
            "until end of turn: the Forests are plain lands again"
        );
    }
}
