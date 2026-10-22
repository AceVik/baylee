//! `cards/creatures/mv_3/zhalfirin_commander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zhalfirin_commander() -> CardIndex {
    card_index("1053a778-da6a-4b8e-81fb-537a355d4bfe")
}

/// Zhalfirin Commander (`{2}{W}`, `Coverage::Implemented`): 2/2 Human Knight
/// with flanking and `{1}{W}{W}: Target Knight creature gets +1/+1 until end of
/// turn.`
#[test]
fn zhalfirin_commander_pumps_target_knight() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                zhalfirin_commander(),
                grizzly_bears(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let commander = on_battlefield(&engine, p0, zhalfirin_commander()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert!(keywords(&engine, commander).contains(KeywordSet::FLANKING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: commander,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(options, vec![commander], "only Knight creatures are legal");
    assert!(!options.contains(&bear));
    aim_at(&mut engine, p0, commander);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, commander), (3, 3));
}
