//! `cards/creatures/mv_3/tar_pit_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn tar_pit_warrior() -> CardIndex {
    card_index("05a7ca83-e820-433f-b9e9-151e817d3708")
}

/// `Tar Pit Warrior` ({2}{B}, 3/4 Creature — Cyclops Warrior,
/// `Coverage::Implemented`) sacrifices itself when it becomes the target of a
/// spell or ability.
#[test]
fn tar_pit_warrior_sacrifices_itself_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest(), GRIZZLY_BEARS])
        .hand(0, &[tar_pit_warrior(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, tar_pit_warrior());
    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, tar_pit_warrior()).unwrap();

    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, warrior);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, tar_pit_warrior()).is_some());
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Battlefield);
}

/// "When *this creature* becomes the target": another creature being targeted
/// does not sacrifice the Warrior.
#[test]
fn tar_pit_warrior_stays_when_another_creature_is_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest(), GRIZZLY_BEARS])
        .hand(0, &[tar_pit_warrior(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, tar_pit_warrior());
    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, tar_pit_warrior()).unwrap();

    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(warrior).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        pt(&engine, bear),
        (5, 5),
        "Giant Growth resolved on the Bears"
    );
}

/// "A spell or ability", whoever controls it: an opponent's Lightning Bolt aimed
/// at the Warrior sacrifices it and the Bolt does nothing more.
#[test]
fn tar_pit_warrior_is_sacrificed_when_an_opponent_targets_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[mountain()])
        .hand(0, &[tar_pit_warrior()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tar_pit_warrior());
    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, tar_pit_warrior()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, lightning_bolt());
    aim_at(&mut engine, p1, warrior);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, tar_pit_warrior()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
}
