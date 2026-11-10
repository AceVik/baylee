//! `cards/sorceries/mv_4/ancient_craving.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Craving is a {3}{B} sorcery under `Coverage::Implemented` that draws three cards and causes 3 life loss.
/// Casting it from hand expends four black mana from basic lands.
/// Upon resolution, three cards are drawn into the caster's hand and their life total drops from 20 to 17.
/// The opponent's life total remains completely unaffected.
#[test]
fn ancient_craving_draws_three_cards_and_loses_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[ancient_craving()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, ancient_craving()).expect("Ancient Craving is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{3}}{{B}}"
    );

    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps provide four mana"
    );
    cast_with_floating(&mut engine, p0, ancient_craving());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 3,
        "three cards drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "hand size net increased by 2 (3 drawn minus Ancient Craving)"
    );
    assert_eq!(engine.state().players[0].life, 17, "controller lost 3 life");
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent life unchanged"
    );
    assert!(
        in_graveyard(&engine, p0, ancient_craving()).is_some(),
        "Ancient Craving is in graveyard"
    );
}
