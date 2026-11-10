//! `cards/instants/mv_2/terminate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Terminate: "Destroy target creature. It can't be regenerated."
///
/// The second sentence is the half that used to cost nothing. While this
/// engine had no regeneration at all, `destroy` and `destroy_no_regen` were
/// the same function and every card printing the clause was correct for
/// free — so a shield standing on the target when the spell resolves is the
/// only thing that has ever told the two apart (CR 701.19c).
///
/// Its own controller's Troll, because a shield has to be *bought* to mean
/// anything and the Troll's `{B}` is the cheapest one there is. A
/// Terminate wired to the ordinary door would leave the Troll standing
/// here, tapped and unhurt.
#[test]
fn terminate_kills_through_a_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(511, forest())
        .battlefield(0, &[lotleth_troll(), swamp(), swamp(), mountain()])
        .hand(0, &[terminate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, terminate());
    let menu = aim_at(&mut engine, p0, troll);
    assert!(
        menu.contains(&troll),
        "a shielded creature is a legal target like any other: {menu:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, lotleth_troll()).is_none(),
        "the shield did not save it"
    );
    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "and it is in its owner's graveyard"
    );
}
