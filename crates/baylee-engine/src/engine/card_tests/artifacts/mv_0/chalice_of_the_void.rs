//! Chalice of the Void — `{X}{X}` Artifact (MRD).
//! Oracle: "This artifact enters with X charge counters on it. Whenever a player casts a spell with mana value equal to the number of charge counters on this artifact, counter that spell."

use super::*;

fn chalice_of_the_void() -> CardIndex {
    card_index("74027ba4-a6ba-4696-baf1-60ef0b27ee0a")
}

#[test]
fn chalice_of_the_void_enters_with_x_charge_counters_and_counters_matching_mana_value_spells() {
    let p0 = PlayerId::new(0);
    let chalice = chalice_of_the_void();
    let elf = llanowar_elves();
    let ring = sol_ring();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[chalice, elf, ring])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    let chalice_card = in_hand(&engine, p0, chalice).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: chalice_card })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chalice_obj = on_battlefield(&engine, p0, chalice).unwrap();
    assert_eq!(
        engine
            .state()
            .object(chalice_obj)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );

    // Llanowar Elves has mana value 1 -> countered by Chalice at 1 charge counter.
    tap_mana_where(&mut engine, p0, |id| id == forests[2]);
    cast_with_floating(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, elf).is_none());
    assert!(in_graveyard(&engine, p0, elf).is_some());
}
