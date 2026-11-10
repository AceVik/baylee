//! `cards/creatures/mv_5/serra_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serra Angel is a 4/4 Angel with flying and vigilance under `Coverage::Implemented` costing {3}{W}{W}.
/// Both keywords reach the permanent through the continuous layer system.
/// In combat, declaring Serra Angel as an attacker does not cause it to tap, proving the vigilance rule under `CR 702.20b`.
/// In contrast, an attacker without vigilance declared in the same attack step becomes tapped under `CR 508.1f`.
#[test]
fn serra_angel_has_flying_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[serra_angel(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let angel = on_battlefield(&engine, p0, serra_angel()).expect("Serra Angel is on battlefield");
    let elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is on battlefield");

    assert_eq!(
        pt(&engine, angel),
        (4, 4),
        "printed power and toughness is 4/4"
    );
    let kw = keywords(&engine, angel);
    assert!(kw.contains(KeywordSet::FLYING), "Serra Angel has flying");
    assert!(
        kw.contains(KeywordSet::VIGILANCE),
        "Serra Angel has vigilance"
    );
    assert!(
        !is_tapped(&engine, angel) && !is_tapped(&engine, elf),
        "both start untapped"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on declare-attackers")
    };
    assert_eq!(player, p0, "active seat declares attackers");
    assert!(
        attackers.contains(&angel) && attackers.contains(&elf),
        "both creatures are legal attackers: {attackers:?}"
    );
    assert!(
        defenders.contains(&Defender::Player(p1)),
        "defending player can be attacked: {defenders:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1)), (elf, Defender::Player(p1))],
            },
        )
        .expect("both declared as attackers");

    assert!(
        !is_tapped(&engine, angel),
        "CR 702.20b: attacking does not cause Serra Angel to tap because it has vigilance"
    );
    assert!(
        is_tapped(&engine, elf),
        "CR 508.1f: the creature without vigilance taps upon attacking"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player: blocker_player,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected block choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            blocker_player,
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        15,
        "four damage from Serra Angel and one from the Elf"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "attacking player life remains unchanged"
    );
}
