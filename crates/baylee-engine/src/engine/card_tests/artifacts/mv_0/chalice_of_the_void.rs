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

/// On the battlefield X is 0 (CR 202.3e), so a Chalice cast for one is an
/// artifact of mana value zero, whatever X its spell had. Divine Offering
/// destroys it and gains its controller nothing. `Amount::TargetCmc` used to
/// read the X the spell was cast with, which a permanent keeps for its own
/// enter replacement, and gained two.
#[test]
fn a_chalice_on_the_battlefield_has_mana_value_zero() {
    use crate::engine::card_tests::partials_w2c_kit as kit;
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &kit::lands(plains(), 4))
        .hand(
            0,
            &[
                chalice_of_the_void(),
                baylee_core::generated::index::DIVINE_OFFERING,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, kit::P0));
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(
        &mut engine,
        kit::P0,
        chalice_of_the_void(),
        &mut kit::Answers::new().x(1),
    );
    kit::settle(&mut engine, kit::P0, &mut kit::Answers::new());
    let chalice = on_battlefield(&engine, kit::P0, chalice_of_the_void()).expect("Chalice");
    assert_eq!(
        engine
            .state()
            .object(chalice)
            .map(|o| o.characteristics().mana_value()),
        Some(0),
        "a permanent's X is 0"
    );
    kit::cast(
        &mut engine,
        kit::P0,
        baylee_core::generated::index::DIVINE_OFFERING,
        &mut kit::Answers::new().aim(&[chalice]),
    );
    kit::settle(&mut engine, kit::P0, &mut kit::Answers::new());
    assert!(
        on_battlefield(&engine, kit::P0, chalice_of_the_void()).is_none(),
        "Divine Offering destroyed the Chalice"
    );
    assert_eq!(kit::life(&engine, kit::P0), 20, "and gained nothing");
}
