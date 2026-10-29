//! `cards/instants/mv_1/fade_from_memory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn fade_from_memory_exiles_target_card_from_a_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let fade = card_index("7c943883-06af-45a4-b5ed-b4564835dbb0");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fade])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    bury(&mut engine, &[elf]);
    let target = in_graveyard(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, fade);
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p1)).len(), 1);
}

#[test]
fn fade_from_memory_cycles_for_black_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let fade = card_index("7c943883-06af-45a4-b5ed-b4564835dbb0");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[fade])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, fade, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, fade).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
