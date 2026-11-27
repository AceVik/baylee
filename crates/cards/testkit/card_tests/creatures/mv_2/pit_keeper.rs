//! `cards/creatures/mv_2/pit_keeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn pit_keeper() -> CardIndex {
    card_index("e1c5bfae-07cb-4194-8a86-201503e17085")
}

/// Pit Keeper's ETB trigger does not fire with fewer than four creature cards in your graveyard.
#[test]
fn pit_keeper_does_not_trigger_below_four_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                grizzly_bears(),
                llanowar_elves(),
                pearled_unicorn(),
            ],
        )
        .hand(0, &[pit_keeper()])
        .start();
    keep_mulligans(&mut engine);
    let to_bury = vec![
        on_battlefield(&engine, p0, grizzly_bears()).unwrap(),
        on_battlefield(&engine, p0, llanowar_elves()).unwrap(),
        on_battlefield(&engine, p0, pearled_unicorn()).unwrap(),
    ];
    bury(&mut engine, &to_bury);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pit_keeper());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(on_battlefield(&engine, p0, pit_keeper()).is_some());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_none());
}

/// With four or more creature cards in your graveyard, Pit Keeper returns target
/// creature card from your graveyard to your hand.
#[test]
fn pit_keeper_returns_creature_when_four_or_more_in_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                grizzly_bears(),
                grizzly_bears(),
                llanowar_elves(),
                pearled_unicorn(),
            ],
        )
        .hand(0, &[pit_keeper()])
        .start();
    keep_mulligans(&mut engine);
    let mut to_bury = all_on_battlefield(&engine, p0, grizzly_bears());
    to_bury.push(on_battlefield(&engine, p0, llanowar_elves()).unwrap());
    to_bury.push(on_battlefield(&engine, p0, pearled_unicorn()).unwrap());
    bury(&mut engine, &to_bury);
    reach_main_phase(&mut engine, p0);

    let bear_gy = in_graveyard(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, pit_keeper());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_gy],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
}
