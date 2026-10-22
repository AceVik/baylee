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
