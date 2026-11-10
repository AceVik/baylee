//! `cards/creatures/mv_3/zhalfirin_commander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zhalfirin_commander() -> CardIndex {
    card_index("1053a778-da6a-4b8e-81fb-537a355d4bfe")
}

fn searing_spear_askari() -> CardIndex {
    card_index("d3c68573-95b1-4db5-bbea-363b86f713c1")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Zhalfirin Commander (`{2}{W}`, `Coverage::Implemented`): 2/2 Human Knight
/// with flanking and `{1}{W}{W}: Target Knight creature gets +1/+1 until end of
/// turn.` Any Knight is a target, an opponent's too, and a Bears is not.
#[test]
fn zhalfirin_commander_pumps_target_knight() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
        .battlefield(1, &[searing_spear_askari()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let commander = on_battlefield(&engine, p0, zhalfirin_commander()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_knight = on_battlefield(&engine, p1, searing_spear_askari()).unwrap();
    assert!(keywords(&engine, commander).contains(KeywordSet::FLANKING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: commander,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(options.contains(&commander));
    assert!(
        options.contains(&their_knight),
        "any Knight, an opponent's too"
    );
    assert!(!options.contains(&bear), "the Bears is not a Knight");
    assert_eq!(options.len(), 2);
    aim_at(&mut engine, p0, commander);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, commander), (3, 3));
    assert_eq!(
        pt(&engine, their_knight),
        (2, 2),
        "only the target is pumped"
    );
    // Until end of turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, commander), (2, 2));
}

/// Zhalfirin Commander — flanking: a blocker without flanking gets -1/-1, one
/// with flanking (the Askari) does not.
#[test]
fn zhalfirin_commander_flanking_shrinks_only_blockers_without_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[zhalfirin_commander()])
        .battlefield(1, &[grizzly_bears(), searing_spear_askari()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let commander = on_battlefield(&engine, p0, zhalfirin_commander()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let askari = on_battlefield(&engine, p1, searing_spear_askari()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(commander, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, commander), (askari, commander)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1), "no flanking, -1/-1");
    assert_eq!(pt(&engine, askari), (2, 2), "has flanking, untouched");
}
