//! `cards/sorceries/mv_3/yawgmoth_s_will.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A graveyard land becomes playable after resolution, subject to the
/// ordinary land-play limits.
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

#[test]
fn yawgmoths_will_casts_from_the_graveyard_and_exiles_only_your_cards_this_turn() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = Duel::new(958, dark_ritual())
        .hand(0, &[yawgmoth_s_will()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, kit::P0));
    seed_graveyard(&mut engine, kit::P0, 1);
    let ritual = in_graveyard(&engine, kit::P0, dark_ritual()).unwrap();
    let will = in_hand(&engine, kit::P0, yawgmoth_s_will()).unwrap();
    kit::float(&mut engine, kit::P0, &[(ManaColor::Black, 4)]);
    kit::cast_at(&mut engine, kit::P0, yawgmoth_s_will(), &[], &[]);
    assert_eq!(engine.state().object(will).unwrap().zone, Zone::Exile);
    assert!(!crate::casting::flashback_granted(engine.state(), ritual));
    engine
        .apply(kit::P0, PlayerAction::CastSpell { card: ritual })
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(kit::pool_of(&engine, kit::P0, ManaColor::Black), 3);
    assert_eq!(engine.state().object(ritual).unwrap().zone, Zone::Exile);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(kit::P0))
        .last()
        .unwrap();
    seed_graveyard(&mut engine, kit::P0, 1);
    assert_eq!(engine.state().object(top).unwrap().zone, Zone::Exile);
    seed_graveyard(&mut engine, kit::P1, 1);
    assert!(in_graveyard(&engine, kit::P1, dark_ritual()).is_some());
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    seed_graveyard(&mut engine, kit::P0, 1);
    assert!(in_graveyard(&engine, kit::P0, dark_ritual()).is_some());
}
