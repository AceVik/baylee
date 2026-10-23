//! `cards/creatures/mv_2/benevolent_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn benevolent_unicorn() -> CardIndex {
    card_index("b437c963-def5-40d6-b567-ae9f1d2a0fa5")
}

#[test]
fn benevolent_unicorn_enters_as_one_two_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[benevolent_unicorn()])
        .battlefield(0, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, benevolent_unicorn());
    pass_until(&mut engine, stack_is_empty);

    let unicorn = on_battlefield(&engine, p0, benevolent_unicorn()).unwrap();
    assert_eq!(pt(&engine, unicorn), (1, 2));
}

/// Benevolent Unicorn (Partial): "If a spell would deal damage to a permanent
/// or player, it deals that much damage minus 1 instead." Not played, so a
/// spell deals its full damage. A pinned limitation: this test moves when the
/// reduction can be said.
#[test]
fn benevolent_unicorn_does_not_yet_reduce_spell_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[benevolent_unicorn(), mountain()])
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17);
}
