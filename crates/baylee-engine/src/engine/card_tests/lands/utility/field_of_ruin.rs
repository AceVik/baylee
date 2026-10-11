//! `cards/lands/utility/field_of_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Field of Ruin prints `{{T}}: Add {{C}}.` and `{{2}}, {{T}}, Sacrifice this land: Destroy target nonbasic land an opponent controls. Each player searches their library for a basic land card, puts it onto the battlefield, then shuffles.`
///
/// With `{{2}}` floating from two `forest()` lands, Field of Ruin untapped, and an opponent controlling
/// both `badlands()` and `forest()`, ability 1 targets only the opponent's nonbasic land.
/// After choosing the target, the sacrifice and mana costs are paid, `badlands()` is destroyed,
/// and each player finds an untapped basic land.
#[test]
fn field_of_ruin_destroys_opponent_nonbasic_land_and_each_player_searches() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[field_of_ruin(), forest(), forest()])
        .battlefield(1, &[badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let field = on_battlefield(&engine, p0, field_of_ruin()).expect("field of ruin on battlefield");
    let target_badlands = on_battlefield(&engine, p1, badlands()).expect("badlands on battlefield");
    let opponent_forest =
        on_battlefield(&engine, p1, forest()).expect("opponent forest on battlefield");

    // Float {{2}} from the two Forests while keeping Field of Ruin untapped.
    tap_mana_except(&mut engine, p0, field);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, field));

    activate(&mut engine, p0, field_of_ruin(), 1);

    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };

    assert!(player_options.is_empty());
    assert!(
        options.contains(&target_badlands),
        "opponent's nonbasic land is a legal target"
    );
    assert!(
        !options.contains(&opponent_forest),
        "opponent's basic land is not a legal target"
    );
    assert!(
        !options.contains(&field),
        "controller's own land is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target_badlands],
                players: vec![],
            },
        )
        .unwrap();

    // Costs paid: Field of Ruin sacrificed, {{2}} deducted from pool.
    assert!(
        in_graveyard(&engine, p0, field_of_ruin()).is_some(),
        "field of ruin sacrificed as cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    for player in [p0, p1] {
        pass_until(
            &mut engine,
            crate::engine::card_tests::library_choices::searched,
        );
        assert_eq!(engine.pending().asked(), Some(player));
        let Pending::ChooseCards { options, .. } = engine.pending() else {
            unreachable!()
        };
        let found = options[0];
        crate::engine::card_tests::library_choices::choose(&mut engine, vec![found]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, badlands()).is_some(),
        "opponent's badlands destroyed"
    );
    assert!(
        engine.state().object(opponent_forest).is_some(),
        "opponent's forest remains"
    );

    // Each search found one basic land.
    assert_eq!(library_size(&engine, p0), 59);
    assert_eq!(library_size(&engine, p1), 59);
}
