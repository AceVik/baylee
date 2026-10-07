//! `cards/instants/mv_3/feeding_frenzy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Feeding Frenzy: "-X/-X, where X is the number of Zombies **on the
/// battlefield**".
///
/// The mirror of the card above, and the reason both are here. `Count$Valid
/// Zombie` names no controller, and the transcoder writes that as a
/// `CountOf` over `ZoneSel::Battlefield` — which counts everybody's. A reader
/// that narrowed it to "you control" would be wrong in the direction nothing
/// complains about: the spell still shrinks, just by less.
#[test]
fn feeding_frenzy_counts_every_zombie_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(312, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[feeding_frenzy()])
        .battlefield(1, &[festering_goblin(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, feeding_frenzy());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (3, 3),
        "one Zombie of yours and two of theirs is three Zombies: a 6/6 \
         becomes a 3/3, and a count that stopped at your own side would \
         leave a 5/5"
    );
}

/// CR 608.2f: a counted amount is read **on resolution**, not when the spell
/// was announced.
///
/// The half a fixed number cannot be wrong about, and the one `Amount::CountOf`
/// makes reachable: `resolve::counters::signed` evaluates through
/// `eval::amount`, which walks the battlefield the engine has at the moment
/// the effect runs. Three Zombies are on the board when Feeding Frenzy is
/// cast and two when it resolves, because the opponent exiled one of their
/// own in response — so the wurm loses two, not three. A reader that had
/// evaluated the count at announcement would pass every other test in this
/// file.
#[test]
fn a_counted_pump_is_read_on_resolution_and_not_on_announcement() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(314, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                festering_goblin(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[feeding_frenzy()])
        .battlefield(1, &[plains(), festering_goblin(), festering_goblin()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, feeding_frenzy());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    // Three Zombies are on the battlefield right now. The spell is on the
    // stack and has counted nothing yet.
    let theirs = all_on_battlefield(&engine, p1, festering_goblin());
    assert_eq!(theirs.len(), 2, "two Zombies on their side, one on ours");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster holds priority over their own spell");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // They answer by exiling one of their own — which is not a Zombie dying,
    // so nothing else goes on the stack.
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (4, 4),
        "three Zombies when it was cast and two when it resolved: a 6/6 \
         becomes a 4/4 (CR 608.2f). A count taken at announcement would have \
         made it a 3/3"
    );
}
