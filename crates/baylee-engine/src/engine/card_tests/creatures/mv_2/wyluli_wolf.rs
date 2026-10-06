//! `cards/creatures/mv_2/wyluli_wolf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wyluli Wolf is a `{1}{G}` 1/1 whose whole text is "{T}: Target creature
/// gets +1/+1 until end of turn." The scenario gives that filter something to
/// *decline* on both sides of the table — an Elf under the same seat and an
/// Elf across it — and reads the printed word "creature" and not "creature you
/// control": both are offered, the Wolf itself is offered too, and only the
/// one that is named changes. `(2, 2)` against two untouched `(1, 1)`s is the
/// pump, and the same Elf reading `(1, 1)` again on the opponent's turn is the
/// "until end of turn" the card prints.
#[test]
fn wyluli_wolf_taps_to_pump_the_creature_it_names_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wyluli_wolf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // {1}{G} off the Forests, which `cast_from_hand` taps before it casts: the
    // Wolf has to be a permanent on the table and not a card in a hand.
    cast_from_hand(&mut engine, p0, wyluli_wolf());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wolf = on_battlefield(&engine, p0, wyluli_wolf()).expect("the Wolf resolved");
    assert_eq!(pt(&engine, wolf), (1, 1), "a printed 1/1");
    assert!(!is_tapped(&engine, wolf), "and it enters untapped");
    assert_eq!(pt(&engine, mine), (1, 1), "nothing is pumped yet");

    // Ability 0 is the only line the card prints: its whole price is its own
    // {T}, and it wants one target creature.
    // CR 302.6: a creature that arrived this turn cannot pay a `{T}`, so
    // the turn goes round once before the line is pressed. Both halves are
    // needed — `walk_to_own_main` on its own returns where it stands,
    // because this already *is* p0's own main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    activate(&mut engine, p0, wyluli_wolf(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&wolf),
        "the Wolf is a creature too, so its own ability may point at it — \
         the card says \"target creature\", not \"another\": {options:?}"
    );
    assert!(
        !is_tapped(&engine, wolf),
        "CR 601.2h: the tap is the last step of the activation, so the Wolf is \
         still standing while the question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the one that gets pumped");
    assert!(
        is_tapped(&engine, wolf),
        "paying the {{T}} is what leaves the Wolf tapped"
    );
    pass_until(&mut engine, |e| pt(e, mine) == (2, 2));

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf across the table was a legal target and was not the one named"
    );
    assert_eq!(
        pt(&engine, wolf),
        (1, 1),
        "and the Wolf neither pumps itself nor keeps what it hands out"
    );

    // "until end of turn": by the opponent's main phase the cleanup step has
    // taken it back, so the Elf is the printed 1/1 again.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump ended with the turn it was applied in"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing ever reached that side"
    );
}
