//! `cards/artifacts/mv_4/isolation_cell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn isolation_cell() -> CardIndex {
    card_index("56c558f3-535d-49aa-a2de-54e0bb598483")
}

/// An opponent's creature spell makes them lose 2 life unless they pay {2}.
/// The probe: the opponent casts Llanowar Elves ({G}) with spare Forests.
fn opponent_casts_an_elf(spare: usize) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut lands = vec![forest()];
    lands.extend(vec![forest(); spare]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[isolation_cell()])
        .battlefield(1, &lands)
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let _ = p0;
    engine
}

#[test]
fn an_opponent_who_cannot_pay_loses_two_life() {
    let p1 = PlayerId::new(1);
    let mut engine = opponent_casts_an_elf(0);
    let before = life_of(&engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. }) || e.state().stack.len() > 1
    });
    panic!("PROBE {:?}", engine.pending());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), before - 2);
}
