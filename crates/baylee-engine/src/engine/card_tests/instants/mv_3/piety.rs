//! `cards/instants/mv_3/piety.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Piety — {2}{W} instant: "Blocking creatures get +0/+3 until end of turn."
///
/// The block is real, because "blocking" is a state only the declaration
/// creates (CR 509.1g): p1's Ogre attacks, p0 declares the Bear as its
/// blocker, and only then is Piety cast. The Elf beside the Bear is a
/// creature p0 controls that is not blocking, and the Ogre is an attacking
/// creature, so neither gains the three toughness.
///
/// The bonus is measured through the damage that follows — the 2/2 Bear
/// blocks a 2/2 and lives through two points it could not have survived
/// printed — and through p0's next turn, where the same Bear is a printed
/// 2/2 again. CR 611.2c fixes the pumped set as Piety resolves; CR 514.2
/// ends it at cleanup.
#[test]
fn piety_pumps_only_blocking_creatures_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[piety()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1's turn to attack");

    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the attacker");
    let blocker = on_battlefield(&engine, p0, grizzly_bears()).expect("the blocker-to-be");
    let home = on_battlefield(&engine, p0, llanowar_elves()).expect("p0's other creature");

    let blocks = attack_and_collect_blocks(&mut engine, ogre, p0);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == blocker && o.attackers.contains(&ogre)),
        "the Bear is offered as a blocker for the Ogre: {blocks:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, ogre)],
            },
        )
        .expect("the Bear blocks the Ogre");

    // After blocks are declared the active player gets priority first
    // (CR 509.2); pass it and act on p0's own window, with the block made
    // and combat damage still ahead.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "a printed 2/2 before the pump"
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains pay {{2}}{{W}}, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, piety());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, blocker), (2, 5), "the blocking Bear is +0/+3");
    assert_eq!(pt(&engine, home), (1, 1), "the Elf is not blocking");
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "and the attacking Ogre gains nothing"
    );

    // Through combat damage: the Ogre's two points are lethal to a printed
    // 2/2, so the Bear being alive is the toughness Piety added, and the
    // bonus is still on it after the combat that made it a blocker.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the Bear blocked a 2/2 and survived the two damage (CR 704.5g)"
    );
    assert_eq!(
        pt(&engine, blocker),
        (2, 5),
        "the +0/+3 outlasts the combat it was cast in (CR 611.2c)"
    );

    assert!(walk_to_own_main(&mut engine, p0), "p0's next turn");
    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "\"until end of turn\" ended at cleanup (CR 514.2)"
    );
}
