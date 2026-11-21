//! `cards/artifacts/equipment/mv_3/whispersilk_cloak.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn whispersilk_cloak() -> CardIndex {
    card_index("9ad4f730-a18e-4a7c-a468-a926c718c741")
}

/// Whispersilk Cloak (9ad4f730-a18e-4a7c-a468-a926c718c741) — {3} Equipment:
/// "Equipped creature can't be blocked and has shroud."
/// "Equip {2}"
///
/// Grants both Shroud (untargetable by own Giant Growth and opponent's Bolt)
/// and Unblockable (opponent cannot declare blockers against the host, taking
/// full unblocked combat damage).
#[test]
fn whispersilk_cloak_grants_shroud_and_unblockable() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig_vs(
        whispersilk_cloak(),
        jasmine_boreal(),
        3,
        &[giant_growth()],
        &[llanowar_elves(), mountain()],
        &[lightning_bolt()],
    );
    r.equip_keeping(2, 1);

    assert!(
        keywords(&r.engine, r.host).contains(KeywordSet::SHROUD.union(KeywordSet::UNBLOCKABLE)),
        "equipped host has both shroud and unblockable"
    );
    assert_eq!(keywords(&r.engine, r.bystander), KeywordSet::EMPTY);

    // 1. Shroud: own Giant Growth cannot target equipped host.
    cast_with_floating(&mut r.engine, p0, giant_growth());
    let menu = aim_at(&mut r.engine, p0, r.bystander);
    assert!(
        !menu.contains(&r.host),
        "own Giant Growth blocked by Shroud"
    );
    pass_until(&mut r.engine, stack_is_empty);

    // 2. Unblockable: attacking host cannot be blocked by opponent's Llanowar Elves.
    let Pending::ChooseBlockers { blockers, .. } = r.attack_to_the_blocks() else {
        unreachable!("attack_to_the_blocks returns ChooseBlockers")
    };
    assert!(
        blockers.iter().all(|b| !b.attackers.contains(&r.host)),
        "equipped creature can't be blocked: {blockers:?}"
    );
    assert!(
        r.engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(r.theirs, r.host)],
                },
            )
            .is_err()
    );
    r.engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(life_of(&r.engine, p1), 16, "4/5 hit unblocked (20 -> 16)");

    // 3. Shroud: opponent's Lightning Bolt cannot target equipped host.
    reach_their_main_phase(&mut r.engine, p1);
    cast_from_hand(&mut r.engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = r.engine.pending().clone() else {
        panic!("expected ChooseTargets")
    };
    assert!(
        !options.contains(&r.host),
        "opponent's Bolt blocked by Shroud"
    );
}
