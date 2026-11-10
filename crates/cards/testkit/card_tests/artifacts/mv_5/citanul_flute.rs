#[allow(clippy::wildcard_imports)]
use super::*;

fn citanul_flute() -> CardIndex {
    card_index("6b8183bb-20c4-4d45-b944-508a6a874bb3")
}

#[test]
fn citanul_flute_searches_library_for_creature_with_mv_at_most_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                citanul_flute(),
                llanowar_elves(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                sol_ring(),
            ],
        )
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_library_top(&mut engine, p0, llanowar_elves());
    hand_to_library_top(
        &mut engine,
        p0,
        baylee_core::generated::index::GRIZZLY_BEARS,
    );
    hand_to_library_top(&mut engine, p0, sol_ring());
    cast_from_hand(&mut engine, p0, citanul_flute());
    pass_until(&mut engine, stack_is_empty);
    let flute = on_battlefield(&engine, p0, citanul_flute()).expect("seated");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: flute,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert_eq!(
            options.len(),
            1,
            "only the creature with mana value <= X is offered"
        );
        let pick = options[0];
        assert_eq!(
            engine.state().object(pick).unwrap().card.unwrap().index,
            llanowar_elves()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![pick],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
}
