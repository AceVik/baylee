//! `cards/creatures/mv_4/marsh_viper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn marsh_viper() -> CardIndex {
    card_index("5249df56-7571-46c0-882e-cf88e452fb4d")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Marsh Viper (5249df56-7571-46c0-882e-cf88e452fb4d) — "Whenever this
/// creature deals damage to a player, that player gets two poison counters."
#[test]
fn marsh_viper_poisons_the_player_it_damages() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[marsh_viper()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let viper = on_battlefield(&engine, p0, marsh_viper()).expect("the Viper is out");
    let blocks = attack_and_collect_blocks(&mut engine, viper, p1);
    assert!(blocks.is_empty(), "nobody to block");
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 19, "1 combat damage");
    assert_eq!(engine.state().players[1].poison, 2, "two poison counters");
    assert_eq!(engine.state().players[0].poison, 0, "the attacker has none");
}

/// A second hit: the counters add up.
#[test]
fn marsh_viper_poison_accumulates_over_turns() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[marsh_viper()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let viper = on_battlefield(&engine, p0, marsh_viper()).expect("the Viper is out");
    for hits in 1..=2u16 {
        attack_and_collect_blocks(&mut engine, viper, p1);
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .expect("no blocks");
        pass_until(&mut engine, |e| {
            matches!(e.state().turn.phase, Phase::SecondMain)
        });
        assert_eq!(engine.state().players[1].poison, 2 * hits);
        reach_their_main_phase(&mut engine, p1);
        assert!(walk_to_own_main(&mut engine, p0), "p0's next own main");
    }
}

/// Damage to a creature is not damage to a player.
#[test]
fn marsh_viper_blocked_gives_no_poison() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[marsh_viper()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let viper = on_battlefield(&engine, p0, marsh_viper()).expect("the Viper is out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    let blocks = attack_and_collect_blocks(&mut engine, viper, p1);
    assert!(blocks.iter().any(|b| b.blocker == giant), "{blocks:?}");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, viper)],
            },
        )
        .expect("the Giant blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant lives")
            .damage,
        1,
        "the Viper's 1 damage went to the Giant"
    );
    assert_eq!(engine.state().players[1].poison, 0, "no poison");
    assert_eq!(engine.state().players[1].life, 20);
}
