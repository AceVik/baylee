//! `cards/creatures/mv_4/angelfire_crusader.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Angelfire Crusader is a `{3}{W}` 2/3 whose whole printed text is one line:
/// "`{R}`: This creature gets +1/+0 until end of turn."
///
/// Every word of that line needs the board to say something. The `{R}` is a
/// real price — the offer is read with the red already floating, because
/// `can_afford` reads the pool and not the untapped Mountains, and the pool
/// is read again afterwards, one red lighter. "This creature" is not
/// "creatures you control": a second creature under the same seat must still
/// read its printed 1/1 while the Crusader is a 3/3. And "until end of turn"
/// is the half a single main phase cannot see, so the body is read a third
/// time after a whole turn cycle has gone by.
#[test]
fn angelfire_crusader_pays_red_to_pump_itself_and_only_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                angelfire_crusader(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let crusader = on_battlefield(&engine, p0, angelfire_crusader()).expect("the Crusader is out");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, crusader), (2, 3), "the body the card prints");
    assert_eq!(pt(&engine, bystander), (1, 1), "and the Elf beside it");

    // Two Mountains pay the {R}, and the Elves are named as the source kept
    // back so that "one red" below is a claim about the lands and not about a
    // creature that tapped for mana on the way.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(crusader, 0)),
        "with {{R}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, angelfire_crusader(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the activation spent one of the two red"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, crusader),
        (3, 3),
        "+1/+0 on the creature the ability names"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "\"this creature\" is not \"creatures you control\""
    );

    // A whole turn cycle: the pump is until end of turn and nothing longer.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, crusader),
        (2, 3),
        "\"until end of turn\": the printed body is back once the turn has turned"
    );
}
