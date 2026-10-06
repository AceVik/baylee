//! `cards/artifacts/mv_4/conduit_of_worlds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Conduit of Worlds: "You may play lands from your graveyard."
///
/// A Forest in the graveyard is offered as a land play and played from
/// there.
#[test]
fn conduit_of_worlds_plays_a_land_from_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conduit_of_worlds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_forest = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.contains(&gy_forest),
        "graveyard land offered as legal land play"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_forest })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest entered battlefield from graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest no longer in graveyard"
    );
}

/// Conduit of Worlds' cast, said yes to and paid short: Canopy Spider's
/// `{1}{G}` with one Forest tapped in the window. Passing there leaves the
/// pool short, so the cast's own window opens for the rest (the wizard's
/// Done stage); passing short again casts nothing, the Spider stays in the
/// graveyard, and the play begun is reversed (CR 732.1): the Forest tapped
/// in the first window untaps and its {G} leaves the pool. The second
/// window carries the first one's start, so both are given back together.
#[test]
fn conduit_of_worlds_paid_short_gives_back_the_forest_tapped_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conduit_of_worlds(), forest(), forest()])
        .hand(0, &[canopy_spider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let spider = hand_to_graveyard(&mut engine, p0, canopy_spider());

    conduit_targets(&mut engine, spider);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the window: {:?}", engine.pending())
    };
    let tapped = legal.mana_abilities[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: tapped })
        .unwrap();
    assert!(is_tapped(&engine, tapped));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        engine.payment_window().is_some(),
        "the cast's own window opens for what the pool lacks"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    assert!(engine.payment_window().is_none());
    assert_eq!(
        engine.state().object(spider).map(|o| o.zone),
        Some(Zone::Graveyard),
        "nothing was cast"
    );
    assert!(
        !is_tapped(&engine, tapped),
        "the Forest tapped for it untaps"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and its mana is taken back"
    );
}
