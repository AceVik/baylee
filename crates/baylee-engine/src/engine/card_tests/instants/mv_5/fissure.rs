//! `cards/instants/mv_5/fissure.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fissure: "Destroy target creature or land. It can't be regenerated."
///
/// The same clause on a wider filter, and the filter is the half worth
/// asserting beside it: the menu holds the Mountains as well as the Troll,
/// which is what "creature **or land**" means and what a copy of Terminate
/// would not do.
#[test]
fn fissure_reaches_a_land_as_well_and_still_ignores_a_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(512, forest())
        .battlefield(
            0,
            &[
                lotleth_troll(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[fissure()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    let a_mountain = on_battlefield(&engine, p0, mountain()).expect("a Mountain is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, fissure());
    let menu = aim_at(&mut engine, p0, troll);
    assert!(menu.contains(&troll), "the creature half of the filter");
    assert!(
        menu.contains(&a_mountain),
        "and the land half, which is the whole difference from Terminate: {menu:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "the shield did not save it"
    );
}
