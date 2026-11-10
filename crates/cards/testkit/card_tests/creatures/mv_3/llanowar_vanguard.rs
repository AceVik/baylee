//! `cards/creatures/mv_3/llanowar_vanguard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Vanguard is a `{2}{G}` 1/1 Dryad printing one line: "{T}: This
/// creature gets +0/+4 until end of turn." The word that needs a witness is
/// *This* — a second creature under the same seat is what tells a self-pump
/// from a board pump, and `(1, 5)` on the host with `(1, 1)` on the Elf beside
/// it is the only pair of numbers that applies the printed toughness half
/// without inventing the power half or reaching across the seat. The Elves are
/// left untapped on purpose, so the three Forests are exactly the mana the
/// `{2}{G}` was cast off and the offer below is read off a board rather than
/// off a source that might still be hiding green. The turn cycle at the end is
/// the "until end of turn" the card actually prints.
#[test]
fn llanowar_vanguard_taps_for_four_toughness_of_its_own_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[llanowar_vanguard()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    // `{2}{G}` out of the three Forests, with the Elves named as the printing
    // kept back: they are the creature the pump must leave alone, and a mana
    // creature tapped for the cast would be a fourth green nothing here counts.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Elves contributed nothing"
    );
    cast_with_floating(&mut engine, p0, llanowar_vanguard());
    pass_until(&mut engine, stack_is_empty);
    let vanguard = on_battlefield(&engine, p0, llanowar_vanguard())
        .expect("the Vanguard resolved onto the table");
    assert_eq!(
        pt(&engine, vanguard),
        (1, 1),
        "a printed 1/1 before its own tap"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and the Elf beside it is untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the card arrived by being paid for, not by a source still floating"
    );

    // CR 302.6: a creature that arrived this turn cannot pay a `{{T}}`, so
    // the turn goes round once before the line is pressed. Both halves are
    // needed — `walk_to_own_main` on its own returns where it stands.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    // The whole price of the one line the card prints is its own {T}, so it is
    // offered with an empty pool and there is nothing to tap for it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vanguard, 0)),
        "{{T}}: This creature gets +0/+4 is offered on an empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, llanowar_vanguard(), 0);
    assert!(
        is_tapped(&engine, vanguard),
        "{{T}} is the whole cost and the Vanguard paid it itself"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it is on the stack before it happens"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, vanguard),
        (1, 5),
        "+0/+4 on the creature that tapped — a (5, 5) would be a power the card never grants"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"This creature\" is not \"creatures you control\": the Elf is still the 1/1 it was printed as"
    );

    // "Until end of turn" is half the sentence, and the untap step read beside
    // it is what says the step ran rather than the game having stopped.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Vanguard's controller takes another turn"
    );
    assert_eq!(
        pt(&engine, vanguard),
        (1, 1),
        "\"until end of turn\" — the toughness is gone once that turn has ended"
    );
    assert!(
        !is_tapped(&engine, vanguard),
        "and the untap step stood it back up, so the tap is not what was read"
    );
}
