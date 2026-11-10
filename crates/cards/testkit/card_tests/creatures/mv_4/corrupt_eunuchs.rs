//! `cards/creatures/mv_4/corrupt_eunuchs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Corrupt Eunuchs — {3}{R}, a 2/2 Human Advisor: "When this creature enters,
/// it deals 2 damage to target creature."
///
/// The scenario plays the trigger at the only moment it exists: the spell is
/// cast, the body arrives, and the trigger asks for a target *while the
/// Eunuchs is already standing on the battlefield* (CR 603.3d) — so a board
/// with one Elf on each side is what shows the printed "target creature" is
/// any creature and not "a creature you control". The Elf across the table
/// takes exactly two damage and dies, while the Elf under the same seat as
/// the Eunuchs is the bystander that proves the damage went where it was
/// aimed: it is still a live 1/1 and p1's life never moved.
#[test]
fn corrupt_eunuchs_deals_two_damage_to_the_creature_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[corrupt_eunuchs()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "a printed 1/1 to be killed by two"
    );

    // {3}{R} off the four Mountains; the Elf beside them is named as the
    // printing kept back, because it is the bystander this test reads back
    // afterwards and a creature tapped for its own mana is a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, corrupt_eunuchs());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target question")
    };
    assert_eq!(
        player, p0,
        "the controller of the creature aims its trigger"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger demands one"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, corrupt_eunuchs()).is_some(),
        "the target is chosen as the trigger goes on the stack (CR 603.3d), \
         so the Eunuchs has already entered"
    );
    assert_eq!(
        pt(
            &engine,
            on_battlefield(&engine, p0, corrupt_eunuchs()).expect("checked above")
        ),
        (2, 2),
        "the body the card prints, and no damage pointed at itself"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature that was named left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the trigger did not name never moved"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "it is still the 1/1 it was printed as, with nothing on it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature and never to the player whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, corrupt_eunuchs()).is_some(),
        "a creature whose enters-trigger fired is still on the battlefield"
    );
}
