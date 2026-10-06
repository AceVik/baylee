//! `cards/sorceries/mv_3/pillage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pillage: "Destroy target artifact or land. It can't be regenerated."
///
/// The clause on a card that names no creature at all, which is the case it
/// looks pointless in until an animated land is standing there. Spawning
/// Pool's `{1}{B}` turns it into a 1/1 Skeleton that keeps every land type
/// it had — so it is a legal target for Pillage *and* it can shield itself,
/// and those two facts meeting is the only board on which this sentence of
/// Pillage does any work.
///
/// Four Swamps and three Mountains: the animation takes `{1}{B}`, the
/// granted regeneration another `{B}`, and `mana_pay::pay_any` settles a
/// generic symbol out of `ManaColor::ALL` in order, so each `{1}` reaches
/// for the black before the red.
#[test]
fn pillage_destroys_an_animated_land_that_shielded_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(513, forest())
        .battlefield(
            0,
            &[
                spawning_pool(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[pillage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let pool = on_battlefield(&engine, p0, spawning_pool()).expect("the manland is seated");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, spawning_pool(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    let types = engine.state().object(pool).unwrap().characteristics().types;
    assert!(
        types.contains(TypeSet::CREATURE) && types.contains(TypeSet::LAND),
        "a Skeleton that is still a land, which is what puts it in Pillage's menu"
    );

    raise_a_shield(&mut engine, p0, pool, crate::choice::GRANTED_ABILITY);

    cast_with_floating(&mut engine, p0, pillage());
    let menu = aim_at(&mut engine, p0, pool);
    assert!(menu.contains(&pool), "an animated land is a land: {menu:?}");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, spawning_pool()).is_none(),
        "the shield the land bought itself did not save it"
    );
    assert!(
        in_graveyard(&engine, p0, spawning_pool()).is_some(),
        "and the land is in its owner's graveyard"
    );
}
