//! `cards/sorceries/mv_2/damn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Damn, overloaded: "Destroy target creature. A creature destroyed this way
/// can't be regenerated." with "target" read as "each" (CR 702.96a).
///
/// The overload is the sweeping half, so the clause it carries is
/// `destroy_all_no_regen` rather than `destroy_no_regen` — a second door,
/// written the same day and just as able to be wired to the wrong one. The
/// Elves beside the Troll are what say the mode was overloaded at all:
/// nothing was targeted, and both creatures die.
#[test]
fn damn_overloaded_sweeps_a_shielded_creature_away_with_an_unshielded_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(514, forest())
        .battlefield(
            0,
            &[
                lotleth_troll(),
                llanowar_elves(),
                // Three Swamps and not two. The shield eats a black, and
                // with one left the normal mode's {B}{B} is unaffordable —
                // the engine then has one legal mode, asks nothing, and the
                // choice this test is about never happens.
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[damn()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, damn());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("a modal spell asks which mode, got {:?}", engine.pending())
    };
    assert!(
        options
            .iter()
            .any(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(0))),
        "the normal mode is affordable too, so the overload below is a \
         choice and not the only thing left: {options:?}"
    );
    let overload = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(1)))
        .expect("the overload is the second mode and it is affordable");
    engine
        .apply(p0, PlayerAction::ChooseMode(overload))
        .expect("the overload cost is floating");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the unshielded creature died, which is the sweep happening at all"
    );
    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "and the shielded one died with it"
    );
}
