//! `cards/lands/utility/goblin_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin-town prints `This land enters tapped`, `{T}: Add {B} or {R}`, and `{2}{B}{R}, {T},
/// Sacrifice this land: Put two +1/+1 counters on target Goblin or Orc you control. Activate only as a sorcery.`
/// The card is marked `Coverage::Implemented`.
/// After entering tapped and untapping on the following turn cycle, floating `{2}{B}{R}` allows activating
/// ability index 1 to sacrifice the land and place two `CounterKind::P1P1` counters on controlled `festering_goblin`.
#[test]
fn goblin_town_sacrifices_to_put_two_counters_on_controlled_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), mountain(), mountain(), festering_goblin()],
        )
        .hand(0, &[goblin_town()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let town = play_land(&mut engine, p0, goblin_town());
    assert!(entered_tapped(&engine, town));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, town));

    let gob = on_battlefield(&engine, p0, festering_goblin()).expect("goblin exists");
    assert_eq!(pt(&engine, gob), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(goblin_town()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, goblin_town(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&gob));
    assert!(on_battlefield(&engine, p0, goblin_town()).is_some());

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gob],
                players: vec![],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, goblin_town()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, gob, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, gob), (3, 3));
}
