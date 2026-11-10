//! `cards/instants/mv_2/terror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Terror: "Destroy target creature that isn't artifact or black. That
/// creature can't be regenerated." A shield bought and spent on Thrun does
/// not save it; a black Troll and an artifact Golem beside it are never
/// legal targets at all.
#[test]
fn terror_destroys_through_a_regeneration_shield_and_excludes_black_and_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                thrun_the_last_troll(),
                lotleth_troll(),
                obsianus_golem(),
                forest(),
                forest(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[terror()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let thrun = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("seated");
    let black_troll = on_battlefield(&engine, p0, lotleth_troll()).expect("seated");
    let golem = on_battlefield(&engine, p0, obsianus_golem()).expect("seated");

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    raise_a_shield(&mut engine, p0, thrun, 0);

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, terror());
    let menu = aim_at(&mut engine, p0, thrun);
    assert!(
        menu.contains(&thrun),
        "a nonartifact, nonblack creature: legal"
    );
    assert!(
        !menu.contains(&black_troll),
        "black — \"isn't … black\" excludes it: {menu:?}"
    );
    assert!(
        !menu.contains(&golem),
        "an artifact creature — \"isn't artifact\" excludes it: {menu:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_none(),
        "the shield did not save it"
    );
    assert!(in_graveyard(&engine, p0, thrun_the_last_troll()).is_some());
}
