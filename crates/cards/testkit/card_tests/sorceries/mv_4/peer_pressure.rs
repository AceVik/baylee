//! `cards/sorceries/mv_4/peer_pressure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn peer_pressure() -> CardIndex {
    card_index("bd6b02f9-277e-4f6a-b0a4-d893a939cf73")
}

/// In a three-player game, Peer Pressure steals every creature of the chosen
/// type indefinitely when its caster controls strictly more creatures of that
/// type than each other player, while leaving other types untouched.
#[test]
fn peer_pressure_gains_control_indefinitely_when_strictly_ahead_of_all_others() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, island(), 3)
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .battlefield(2, &[llanowar_elves(), festering_goblin()])
        .hand(0, &[peer_pressure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let p2_elf = on_battlefield(&engine, p2, llanowar_elves()).expect("P2 Elf");
    let p2_goblin = on_battlefield(&engine, p2, festering_goblin()).expect("P2 Goblin");

    cast_from_hand(&mut engine, p0, peer_pressure());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Peer Pressure");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().object(p1_elf).unwrap().controller, p0);
    assert_eq!(engine.state().object(p2_elf).unwrap().controller, p0);
    assert_eq!(
        engine.state().object(p2_goblin).unwrap().controller,
        p2,
        "other creature types are untouched"
    );

    // Control change is indefinite: persists past end of turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().object(p1_elf).unwrap().controller, p0);
    assert_eq!(engine.state().object(p2_elf).unwrap().controller, p0);
}

/// If any other player ties the caster's count (even when another seat has fewer),
/// Peer Pressure does nothing.
#[test]
fn peer_pressure_does_nothing_when_tied_with_any_other_player() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, island(), 3)
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[])
        .battlefield(2, &[llanowar_elves()])
        .hand(0, &[peer_pressure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p2_elf = on_battlefield(&engine, p2, llanowar_elves()).expect("P2 Elf");
    cast_from_hand(&mut engine, p0, peer_pressure());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().object(p2_elf).unwrap().controller,
        p2,
        "1 vs (0, 1) is tied with P2, not strictly more than each other player"
    );
    let _ = p1;
}
