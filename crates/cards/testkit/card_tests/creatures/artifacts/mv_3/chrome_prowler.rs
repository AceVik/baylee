//! `cards/creatures/artifacts/mv_3/chrome_prowler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chrome_prowler() -> CardIndex {
    card_index("6d16c2fa-4118-4b81-92ca-6c8e9efe03b6")
}

/// Chrome Prowler — "When this creature enters, tap target creature an
/// opponent controls": the menu holds the opponent's creature and neither the
/// caster's own nor the Prowler, and the target ends up tapped.
#[test]
fn chrome_prowler_taps_target_opposing_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[chrome_prowler()])
        .battlefield(0, &[island(), island(), island(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert!(!is_tapped(&engine, theirs));

    cast_from_hand(&mut engine, p0, chrome_prowler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, theirs);
    assert!(options.contains(&theirs));
    assert!(!options.contains(&mine), "only an opponent's creature");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, theirs), "the target was tapped");
    let prowler = on_battlefield(&engine, p0, chrome_prowler()).expect("it resolved");
    assert_eq!(pt(&engine, prowler), (3, 2));
}

/// Flash: the Prowler is cast in seat 1's own main phase, after seat 1 passes
/// priority, which no sorcery-speed creature may do.
#[test]
fn chrome_prowler_has_flash_and_is_cast_on_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[chrome_prowler()])
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "seat 0 now holds priority in seat 1's main phase"
    );

    cast_from_hand(&mut engine, p0, chrome_prowler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    aim_at(&mut engine, p0, theirs);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, chrome_prowler()).is_some());
    assert!(is_tapped(&engine, theirs));
}
