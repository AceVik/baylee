//! `cards/artifacts/mv_1/tel_jilad_stylus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tel_jilad_stylus() -> CardIndex {
    card_index("75a7fdd4-046a-433b-b16f-a0424a05c904")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

/// Tel-Jilad Stylus (75a7fdd4-046a-433b-b16f-a0424a05c904) — {1} Artifact:
/// "{T}: Put target permanent you own on the bottom of your library."
///
/// Offers only permanents you own (excluding the opponent's) and places the
/// target at the bottom of your library.
#[test]
fn tel_jilad_stylus_tucks_owned_permanent_to_bottom_and_excludes_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tel_jilad_stylus(), grizzly_bears()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stylus = on_battlefield(&engine, p0, tel_jilad_stylus()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let lib_before = library_size(&engine, p0);

    activate(&mut engine, p0, tel_jilad_stylus(), 0);
    let menu = aim_at(&mut engine, p0, bears);
    assert!(
        menu.contains(&bears) && menu.contains(&stylus),
        "permanents you own are offered: {menu:?}"
    );
    assert!(
        !menu.contains(&their_elves),
        "permanents owned by the opponent are excluded: {menu:?}"
    );

    assert!(is_tapped(&engine, stylus), "activating taps Stylus");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, p0, grizzly_bears()), None);
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(lib.len(), lib_before + 1);
    assert_eq!(
        lib.first().copied(),
        Some(bears),
        "targeted permanent lands on the bottom of its owner's library"
    );
}

/// "Target permanent you own" still matches a permanent you own even after
/// the opponent steals control of it with Control Magic.
#[test]
fn tel_jilad_stylus_tucks_owned_permanent_stolen_by_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[tel_jilad_stylus(), grizzly_bears()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[control_magic()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p1, control_magic());
    aim_at(&mut engine, p1, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        p1,
        "opponent now controls Grizzly Bears"
    );

    let aura = on_battlefield(&engine, p1, control_magic()).unwrap();
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, tel_jilad_stylus(), 0);
    let menu = aim_at(&mut engine, p0, bears);
    assert!(
        menu.contains(&bears),
        "you still own the stolen Bears, so Stylus can target it: {menu:?}"
    );
    assert!(
        !menu.contains(&aura),
        "opponent's Control Magic is not owned by you: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .copied(),
        Some(bears),
        "stolen Bears returns to the bottom of your library"
    );
}
