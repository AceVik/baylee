//! `cards/artifacts/mv_3/runed_arch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn runed_arch() -> CardIndex {
    card_index("32717625-46c6-45c5-ad0a-fb6a91f9878c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn cast_arch() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
                gray_ogre(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[runed_arch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 1. Cast Runed Arch from hand -> enters tapped.
    cast_from_hand(&mut engine, P0, runed_arch());
    pass_until(&mut engine, stack_is_empty);
    let arch = on_battlefield(&engine, P0, runed_arch()).unwrap();
    assert!(
        is_tapped(&engine, arch),
        "Runed Arch enters the battlefield tapped"
    );

    (engine, arch)
}

/// Runed Arch (32717625-46c6-45c5-ad0a-fb6a91f9878c) — {3} Artifact:
/// - "This artifact enters tapped."
/// - "{X}, {T}, Sacrifice this artifact: X target creatures with power 2 or less can't be blocked this turn."
#[test]
fn runed_arch_enters_tapped_and_makes_x_creatures_with_power_at_most_two_unblockable() {
    let (mut engine, arch) = cast_arch();

    // 2. Advance to P0's next turn (turn 3) so Runed Arch untaps.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(!is_tapped(&engine, arch));

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    // Tap 3 Forests and activate Runed Arch with X = 2.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, runed_arch(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseTargets for Runed Arch, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((min, max), (2, 2));
    assert!(
        options.contains(&bears) && options.contains(&ogre),
        "2/2 creatures are legal targets"
    );
    assert!(
        !options.contains(&giant),
        "Hill Giant (power 3) is excluded from 'power 2 or less'"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, runed_arch()).is_none()
            && in_graveyard(&engine, P0, runed_arch()).is_some(),
        "Runed Arch was sacrificed as part of the activation cost"
    );

    // Attack with Grizzly Bears, Gray Ogre, and Hill Giant: neither 2/2 can be blocked,
    // while Hill Giant CAN be blocked.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears, Defender::Player(P1)),
                    (ogre, Defender::Player(P1)),
                    (giant, Defender::Player(P1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    let blocker_opt = blockers.iter().find(|b| b.blocker == blocker).unwrap();
    assert!(
        !blocker_opt.attackers.contains(&bears) && !blocker_opt.attackers.contains(&ogre),
        "the X targeted creatures with power <= 2 can't be blocked this turn"
    );
    assert!(
        blocker_opt.attackers.contains(&giant),
        "untouched Hill Giant can still be blocked"
    );
}
