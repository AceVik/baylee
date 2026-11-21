//! `cards/artifacts/mv_1/voyager_staff.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn voyager_staff() -> CardIndex {
    card_index("4573f044-7011-412e-81bd-ae8d0c21283b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

/// Voyager Staff (4573f044-7011-412e-81bd-ae8d0c21283b) — {1} Artifact:
/// "{2}, Sacrifice this artifact: Exile target creature. Return the exiled
///  card to the battlefield under its owner's control at the beginning of the
///  next end step."
///
/// Exiles a creature stolen by the opponent (verifying any creature is
/// targetable), keeps it exiled until the end step, and returns it under its
/// owner's control at the beginning of the next end step.
#[test]
fn voyager_staff_exiles_creature_and_returns_under_owners_control_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[voyager_staff(), grizzly_bears(), forest(), forest()])
        .battlefield(
            1,
            &[llanowar_elves(), island(), island(), island(), island()],
        )
        .hand(1, &[control_magic()])
        .start();
    keep_mulligans(&mut engine);

    // Opponent steals p0's Grizzly Bears with Control Magic.
    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p1, control_magic());
    aim_at(&mut engine, p1, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().controller, p1);

    // On p0's turn: pay {2} and sacrifice Voyager Staff targeting the stolen Bears.
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let staff = on_battlefield(&engine, p0, voyager_staff()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, voyager_staff(), 0);
    let menu = aim_at(&mut engine, p0, bears);
    assert!(
        menu.contains(&bears) && menu.contains(&their_elf),
        "any creature on the battlefield is targetable: {menu:?}"
    );
    assert!(
        !menu.contains(&staff),
        "noncreature artifact is not a legal target: {menu:?}"
    );
    assert_eq!(
        engine.state().object(staff).unwrap().zone,
        Zone::Graveyard,
        "Voyager Staff sacrificed as cost"
    );
    pass_until(&mut engine, stack_is_empty);

    // Bears stays in Exile during the main phase.
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
    assert_eq!(on_battlefield(&engine, p0, grizzly_bears()), None);
    assert_eq!(on_battlefield(&engine, p1, grizzly_bears()), None);

    // Cross p0's end step into p1's turn: Bears returned under owner p0's control.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "exiled Bears returned to the battlefield under its owner's (p0's) control"
    );
    assert_eq!(on_battlefield(&engine, p1, grizzly_bears()), None);
}
