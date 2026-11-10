// Oracle checklist: Target creature gets -5/-5 until end of turn.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("13228a22-d63a-4952-a482-4f84da696e88")
}

#[test]
fn oracle_targeted_creature_effect() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[fangren_hunter(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));
    let target = on_battlefield(&game, opponent, fangren_hunter()).unwrap();
    let own = on_battlefield(&game, caster, llanowar_elves()).unwrap();
    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    assert_eq!(pt(&game, target), (4, 4));
    cast_from_hand(&mut game, caster, subject());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("creature target required: {:?}", game.pending())
    };
    assert!(options.contains(&target));
    assert!(options.contains(&own), "own creatures are legal too");
    assert!(!options.contains(&artifact));
    aim_at(&mut game, caster, target);
    pass_until(&mut game, stack_is_empty);
    assert!(in_graveyard(&game, opponent, fangren_hunter()).is_some());
    assert_eq!(pt(&game, own), (1, 1));

    assert!(in_graveyard(&game, caster, subject()).is_some());
}

#[test]
fn nonlethal_shrink_expires_at_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[rootbreaker_wurm()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let target = on_battlefield(&game, p1, rootbreaker_wurm()).unwrap();
    assert_eq!(pt(&game, target), (6, 6));
    cast_from_hand(&mut game, p0, subject());
    aim_at(&mut game, p0, target);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(pt(&game, target), (1, 1));
    reach_their_main_phase(&mut game, p1);
    assert_eq!(pt(&game, target), (6, 6));
}
