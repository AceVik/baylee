//! `cards/creatures/artifacts/mv_6/limestone_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Limestone Golem` prints `{{2}}, Sacrifice this creature: Target player draws a card.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/4 artifact creature and two Forests.
/// Activating the ability asks for the target player first before paying the costs (CR 601.2c, 601.2h).
/// Answering the target choice spends the two mana and sacrifices the creature, and resolution draws a card for the chosen player.
#[test]
fn limestone_golem_targets_player_then_pays_cost_to_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[limestone_golem(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, limestone_golem()).expect("golem is seated");
    assert_eq!(pt(&engine, golem), (3, 4));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, limestone_golem(), 0);

    // CR 601.2c: Target is chosen first; cost is not yet paid.
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        player_options.contains(&p0),
        "seat 0 is a legal target player"
    );
    assert!(
        on_battlefield(&engine, p0, limestone_golem()).is_some(),
        "creature is still on the battlefield while target is chosen"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "mana is still in pool while target is chosen"
    );

    // CR 601.2h: Selecting target triggers cost payment.
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("target player chosen");

    assert!(
        on_battlefield(&engine, p0, limestone_golem()).is_none(),
        "`Limestone Golem` was sacrificed after target selection"
    );
    assert!(
        in_graveyard(&engine, p0, limestone_golem()).is_some(),
        "`Limestone Golem` moved to graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two mana consumed to pay cost"
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);
    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "target player drew exactly one card upon resolution"
    );
}
