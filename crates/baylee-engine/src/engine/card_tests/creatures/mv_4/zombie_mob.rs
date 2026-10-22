//! `cards/creatures/mv_4/zombie_mob.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zombie_mob() -> CardIndex {
    card_index("c56f0f4c-c85a-4931-b22a-42dc5d1dc7dd")
}

/// Zombie Mob (`{2}{B}{B}`, `Coverage::Partial`): 2/0 Zombie that enters with a
/// +1/+1 counter on it for each creature card in your graveyard and exiles your
/// graveyard when it enters.
#[test]
fn zombie_mob_enters_with_counters_for_creatures_in_graveyard_and_exiles_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt(), zombie_mob()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put two creature cards (and two instant cards) into p0's graveyard.
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, zombie_mob());
    pass_until(&mut engine, stack_is_empty);

    let mob = on_battlefield(&engine, p0, zombie_mob()).unwrap();
    // 2 base power + 2 creature cards in graveyard = 4/2 (instants in graveyard are not counted).
    assert_eq!(pt(&engine, mob), (4, 2));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 4);
}
