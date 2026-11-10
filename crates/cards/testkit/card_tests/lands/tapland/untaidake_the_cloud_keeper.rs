//! `cards/lands/tapland/untaidake_the_cloud_keeper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Untaidake, the Cloud Keeper: "Untaidake enters tapped." / "{T}, Pay 2 life: Add {C}{C}. Spend this mana only to cast legendary spells."
/// Activating Untaidake deducts 2 life from the controller and produces two restricted colorless mana.
/// The mana pool receives the restricted mana and the land becomes tapped.
#[test]
fn untaidake_the_cloud_keeper_adds_restricted_mana_paying_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[untaidake_the_cloud_keeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let untaidake =
        on_battlefield(&engine, p0, untaidake_the_cloud_keeper()).expect("Untaidake deployed");
    let life_before = engine.state().players[0].life;

    activate(&mut engine, p0, untaidake_the_cloud_keeper(), 0);

    assert_eq!(engine.state().players[0].life, life_before - 2);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, untaidake));
}
