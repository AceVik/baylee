//! `cards/artifacts/mv_4/otherworld_atlas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn otherworld_atlas() -> CardIndex {
    card_index("8d907a1b-f301-46a3-aa1c-25fb7f29c949")
}

fn untap(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    engine
        .dev_state_mut(PlayerId::new(0))
        .unwrap()
        .set_tapped(id, false);
    engine.refresh_offer();
}

/// "{T}: Put a charge counter on this artifact." and "{T}: Each player draws
/// a card for each charge counter on this artifact." Charge it twice, then
/// draw: both seats take two cards, and the counters stay.
#[test]
fn each_player_draws_a_card_per_charge_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[otherworld_atlas()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let atlas = on_battlefield(&engine, p0, otherworld_atlas()).unwrap();

    activate(&mut engine, p0, otherworld_atlas(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, atlas, CounterKind::Charge), 1);
    untap(&mut engine, atlas);
    activate(&mut engine, p0, otherworld_atlas(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, atlas, CounterKind::Charge), 2);
    untap(&mut engine, atlas);

    let (l0, l1) = (library_size(&engine, p0), library_size(&engine, p1));
    activate(&mut engine, p0, otherworld_atlas(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), l0 - 2, "I draw two");
    assert_eq!(library_size(&engine, p1), l1 - 2, "and so does the opponent");
    assert_eq!(counters_on(&engine, atlas, CounterKind::Charge), 2, "none spent");
}

/// With no counters the draw ability draws nothing.
#[test]
fn an_uncharged_atlas_draws_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[otherworld_atlas()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (l0, l1) = (library_size(&engine, p0), library_size(&engine, p1));
    activate(&mut engine, p0, otherworld_atlas(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!((library_size(&engine, p0), library_size(&engine, p1)), (l0, l1));
}
