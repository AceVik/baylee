//! `cards/sorceries/mv_3/yawgmoth_s_will.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Yawgmoth's Will` (`Coverage::Partial`): "Until end of turn, you may play lands and
/// cast spells from your graveyard. If a card would be put into your graveyard from
/// anywhere this turn, exile that card instead."
///
/// Under `Coverage::Partial`, the continuous grant of `Modifier::PlayLandsFromGraveyard`
/// is implemented for the turn, while casting spells from the graveyard and the exile
/// replacement rule are omitted. The test verifies that a graveyard land is not offered
/// before casting, is offered in `legal.lands` once `Yawgmoth's Will` resolves, and can be
/// successfully played onto the battlefield.
#[test]
fn yawgmoth_s_will_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(477, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[yawgmoth_s_will()])
        .start();
    keep_mulligans(&mut engine);

    seed_graveyard(&mut engine, p0, 1);
    let gy_land = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.lands.contains(&gy_land),
        "before Yawgmoth's Will, graveyard land cannot be played"
    );

    cast_from_hand(&mut engine, p0, yawgmoth_s_will());
    pass_until(&mut engine, stack_is_empty);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after resolution, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.lands.contains(&gy_land),
        "after Yawgmoth's Will resolves, graveyard land is offered in legal.lands"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_land })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest entered the battlefield from graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest is no longer in the graveyard"
    );
}
