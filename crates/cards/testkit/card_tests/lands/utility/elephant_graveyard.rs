//! `cards/lands/utility/elephant_graveyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elephant Graveyard: "{T}: Add {C}." / "{T}: Regenerate target Elephant."
///
/// The two abilities share one `{T}`, so no board can show both being used
/// and the *menu* is what is worth reading instead. The Elf standing beside
/// the Elephant is what gives the subtype filter something to refuse: a
/// list of one on a board holding one creature would say nothing at all.
///
/// This was a pin on the missing shield, and it passed for the wrong
/// reason — its board held no Elephant, so the ability was off the offer
/// for want of a target rather than for want of a rule, and it would have
/// gone on passing after regeneration was written.
#[test]
fn elephant_graveyard_regenerates_an_elephant_and_not_the_elf_beside_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(34, forest())
        .hand(0, &[elephant_graveyard()])
        .battlefield(0, &[wild_elephant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let eg = play_land(&mut engine, p0, elephant_graveyard());
    assert!(!is_tapped(&engine, eg));
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("the 3/3 is seated");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(eg, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        legal.abilities.contains(&(eg, 1)),
        "and so is the regeneration, because an Elephant is there to point it at"
    );

    activate(&mut engine, p0, elephant_graveyard(), 1);
    let menu = aim_at(&mut engine, p0, elephant);
    assert_eq!(
        menu,
        vec![elephant],
        "the Elf is a creature and is not an Elephant, so it is not on the menu"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(elephant)
            .expect("the Elephant is untouched by its own shield")
            .regeneration_shields,
        1,
        "one shield, standing until the cleanup step"
    );
    assert!(
        is_tapped(&engine, eg),
        "and the land paid with the tap the mana ability would have wanted"
    );
}
