//! `cards/creatures/mv_5/flame_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flame Spirit prints one line — "{R}: This creature gets +1/+0 until end of
/// turn" — and its price is a mana rather than its own {T}, which is the whole
/// of what makes the pump repeatable. Eight Mountains pay the {4}{R} that
/// brings the 2/3 to the table and leave exactly three red floating, so the
/// offer is read where the engine reads it (`can_afford` looks at the pool and
/// not at untapped lands), three activations take the Spirit to 5/3 without
/// ever tapping it, and the emptied pool is the control that the line was paid
/// for rather than free. A turn later the printed "until end of turn" has left
/// it the 2/3 it was printed as.
#[test]
fn flame_spirit_spends_a_red_a_piece_to_pump_its_power_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 8])
        .hand(0, &[flame_spirit()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Eight Mountains into the pool first: the cast spends five of them and
    // the three left over are what the offer below is read against.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        8,
        "eight Mountains, eight red, and no other source on this board"
    );
    cast_with_floating(&mut engine, p0, flame_spirit());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let spirit = on_battlefield(&engine, p0, flame_spirit()).expect("the Spirit resolved");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, spirit), (2, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        3,
        "eight red less the {{4}}{{R}} the cast cost"
    );

    // Ability 0 is the only line the card prints, and it is offered because
    // the {R} is already floating — never because the Mountains are untapped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(spirit, 0)),
        "with three red in the pool the pump is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, flame_spirit(), 0);
    assert!(
        !is_tapped(&engine, spirit),
        "the price is a mana and not a printed {{T}}, so the Spirit stays standing"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, spirit), (3, 3), "+1/+0 on the creature itself");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "one red per activation"
    );

    // Repeatable, because nothing about the price was the creature's own tap:
    // two more activations off the two red the cast left behind.
    activate(&mut engine, p0, flame_spirit(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    activate(&mut engine, p0, flame_spirit(), 0);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, spirit),
        (5, 3),
        "three activations, three +1/+0, and no toughness anywhere in the line"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "one red per activation, and the three the cast left are gone"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the Spirit itself and never a creature across the table"
    );

    // The emptied pool is the control: `can_afford` reads the pool, so on this
    // board — every Mountain tapped, nothing floating — the line is unpayable
    // and absent from the offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(spirit, 0)),
        "with nothing floating the {{R}} cannot be paid and the line is not offered: {:?}",
        legal.abilities
    );

    // "until end of turn" is part of the card, so the pump has to be gone by
    // the controller's next main phase while the creature stays on the table.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, spirit),
        (2, 3),
        "the pump lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, flame_spirit()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
