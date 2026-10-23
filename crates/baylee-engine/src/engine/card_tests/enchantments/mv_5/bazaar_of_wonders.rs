//! `cards/enchantments/mv_5/bazaar_of_wonders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn bazaar_of_wonders() -> CardIndex {
    card_index("7fca65b8-01fe-4858-97c6-f97ae82cd801")
}

#[test]
fn bazaar_of_wonders_exiles_all_graveyards_on_etb() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[bazaar_of_wonders()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let g0 = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let g1 = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];

    cast_from_hand(&mut engine, p0, bazaar_of_wonders());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(g0).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(g1).unwrap().zone, Zone::Exile);
    // "When this enchantment enters": cards that reach a graveyard later stay.
    assert!(on_battlefield(&engine, p0, bazaar_of_wonders()).is_some());
    seed_graveyard(&mut engine, p1, 1);
    let later = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];
    assert_eq!(engine.state().object(later).unwrap().zone, Zone::Graveyard);
}

/// Bazaar of Wonders (Partial): "Whenever a player casts a spell, counter it
/// if a card with the same name is in a graveyard or a nontoken permanent
/// with the same name is on the battlefield." Not played, so a second Grizzly
/// Bears resolves. A pinned limitation: this test moves when the clause can
/// be said.
#[test]
fn bazaar_of_wonders_does_not_yet_counter_a_spell_that_shares_a_name() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[GRIZZLY_BEARS])
        .battlefield(0, &[bazaar_of_wonders(), GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(
        in_graveyard(&engine, p0, GRIZZLY_BEARS).is_none(),
        "the second Bears was not countered"
    );
}
