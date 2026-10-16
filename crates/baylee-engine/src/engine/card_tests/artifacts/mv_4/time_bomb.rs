//! `cards/artifacts/mv_4/time_bomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn time_bomb() -> CardIndex {
    card_index("29062094-594d-4557-9496-11a15d7e3bb0")
}

/// "At the beginning of your upkeep, put a time counter on this artifact."
/// Mine, not theirs.
#[test]
fn a_time_counter_arrives_at_each_of_my_upkeeps_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[time_bomb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bomb = on_battlefield(&engine, p0, time_bomb()).unwrap();
    let before = counters_on(&engine, bomb, CounterKind::Time);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        counters_on(&engine, bomb, CounterKind::Time),
        before,
        "their upkeep is not mine"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(counters_on(&engine, bomb, CounterKind::Time), before + 1);
}

/// "{1}, {T}, Sacrifice this artifact: This artifact deals damage equal to
/// the number of time counters on it to each creature and each player."
#[test]
fn it_deals_damage_equal_to_its_time_counters_to_every_creature_and_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[time_bomb(), forest(), llanowar_elves()])
        .battlefield(1, &[gray_ogre(), earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bomb = on_battlefield(&engine, p0, time_bomb()).unwrap();
    set_counters(&mut engine, bomb, CounterKind::Time, 2);
    let (l0, l1) = (life_of(&engine, p0), life_of(&engine, p1));

    tap_mana_except(&mut engine, p0, bomb);
    activate(&mut engine, p0, time_bomb(), 1);
    assert!(in_graveyard(&engine, p0, time_bomb()).is_some(), "sacrificed as a cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), l0 - 2, "each player: me");
    assert_eq!(life_of(&engine, p1), l1 - 2, "each player: them");
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "my 1/1 dies to 2"
    );
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "their 2/2 dies to 2"
    );
    assert!(
        on_battlefield(&engine, p1, earth_elemental()).is_some(),
        "a bigger creature survives the damage"
    );
}
