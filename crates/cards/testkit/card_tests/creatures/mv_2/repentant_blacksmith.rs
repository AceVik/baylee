//! `cards/creatures/mv_2/repentant_blacksmith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Repentant Blacksmith — `{1}{W}` 1/2 Human with "Protection from red"
/// (CR 702.16b: a red spell cannot target it). A red Lightning Bolt is cast
/// at a board holding the Blacksmith beside an unprotected Llanowar Elves,
/// so the target menu says which of the two is reachable and the Bolt then
/// resolves on the one it could name.
#[test]
fn repentant_blacksmith_cannot_be_targeted_by_a_red_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[repentant_blacksmith(), llanowar_elves()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let smith = on_battlefield(&engine, p0, repentant_blacksmith()).expect("seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("a legal, unprotected target");
    assert_eq!(pt(&engine, smith), (1, 2), "the printed body");

    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Lightning Bolt's target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&elves),
        "the unprotected creature is offered: {options:?}"
    );
    assert!(
        !options.contains(&smith),
        "protection from red: a red spell cannot target it (CR 702.16b): {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Bolt resolved on the only creature it could name"
    );
}

/// The damage half of the same keyword (CR 702.16e): a red 1/1 attacker is
/// blocked by the Blacksmith, whose 1 power kills it while the 1 damage
/// coming back is prevented — zero marked damage on a 1/2 that would
/// otherwise show one.
#[test]
fn repentant_blacksmith_prevents_a_red_creatures_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[repentant_blacksmith()])
        .battlefield(1, &[mountain_bandit()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let smith = on_battlefield(&engine, p0, repentant_blacksmith()).expect("seated");
    let bandit = on_battlefield(&engine, p1, mountain_bandit()).expect("a red 1/1");
    assert_eq!(pt(&engine, bandit), (1, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bandit, Defender::Player(p0))],
            },
        )
        .expect("its printed haste is permission, and it has been out since before turn one");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(smith, bandit)],
            },
        )
        .expect("a 1/2 may block a 1/1");

    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, mountain_bandit()).is_some()
    });
    assert_eq!(
        engine
            .state()
            .object(smith)
            .expect("the Blacksmith survives")
            .damage,
        0,
        "the red creature's 1 damage is prevented (CR 702.16e)"
    );
    assert_eq!(pt(&engine, smith), (1, 2), "and nothing shrank it");
    assert!(
        in_graveyard(&engine, p1, mountain_bandit()).is_some(),
        "its own 1 damage killed the 1/1 it blocked"
    );
}
