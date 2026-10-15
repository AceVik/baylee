// Oracle checklist: Target creature gets +2/+2 until end of turn.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("31585036-cdc3-47f5-9fc7-913212826f2a")
}

#[test]
fn oracle_targeted_creature_effect() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[gray_ogre(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));
    let target = on_battlefield(&game, opponent, gray_ogre()).unwrap();
    let own = on_battlefield(&game, caster, llanowar_elves()).unwrap();
    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    assert_eq!(pt(&game, target), (2, 2));
    cast_from_hand(&mut game, caster, subject());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("creature target required: {:?}", game.pending())
    };
    assert!(options.contains(&target));
    assert!(options.contains(&own), "own creatures are legal too");
    assert!(!options.contains(&artifact));
    aim_at(&mut game, caster, target);
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, opponent, gray_ogre()).is_some());
    assert_eq!(pt(&game, own), (1, 1));
    assert_eq!(pt(&game, target), (4, 4));

    assert!(in_graveyard(&game, caster, subject()).is_some());
    reach_their_main_phase(&mut game, opponent);
    assert_eq!(pt(&game, target), (2, 2), "pump expires after the turn");
}
