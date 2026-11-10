//! `cards/creatures/mv_3/man_o_war.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Man-o'-War — {2}{U} 2/2 Jellyfish: "When this creature enters, return
/// target creature to its owner's hand."
///
/// Two printed words are what the board has to separate, and neither is
/// visible in the card file. "Target creature" is not "creatures you
/// control", so an Elf under the same seat stands beside the one across the
/// table and only the second is named; and "its owner's hand" is not the
/// hand of the seat that aimed the bounce, so the Elf is read in the hand of
/// the seat that owns it. The Jellyfish itself stays on the battlefield
/// afterwards, which is what tells a bounce apart from a sweeper.
#[test]
fn man_o_war_bounces_the_creature_it_names_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[man_o_war()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    cast_from_hand(&mut engine, p0, man_o_war());
    // The spell resolves and its enters-trigger asks who is being returned.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(
        player, p0,
        "the Jellyfish's controller aims its own trigger"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
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
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it, \
         not to the seat that aimed the bounce"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "a bounce is not a kill: nothing was put into a graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    let jelly =
        on_battlefield(&engine, p0, man_o_war()).expect("the Jellyfish resolved and stayed");
    assert_eq!(pt(&engine, jelly), (2, 2), "the body the card prints");
}
