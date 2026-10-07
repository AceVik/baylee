//! `cards/creatures/mv_2/auratog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Auratog prints one line — "Sacrifice an enchantment: This creature gets
/// +2/+2 until end of turn" — and the whole card is the menu that line
/// raises. The board is built so a wrong menu cannot pass: one enchantment
/// under the Atog's controller, one under the opponent's, and a Sol Ring
/// under the Atog's controller, so the answer says both that the type is read
/// and that CR 701.21a's "you control" is read beside it. The pump is then
/// read off the printed 1/2, and the eaten enchantment is followed to its
/// owner's graveyard rather than merely off the battlefield.
#[test]
fn auratog_eats_an_enchantment_you_control_and_grows_by_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), fastbond(), quiet_artifact()],
        )
        .battlefield(1, &[fastbond()])
        .hand(0, &[auratog()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, auratog());
    pass_until(&mut engine, stack_is_empty);
    let atog = on_battlefield(&engine, p0, auratog()).expect("the Atog resolved");
    let fodder = on_battlefield(&engine, p0, fastbond()).expect("the enchantment is out");
    let theirs = on_battlefield(&engine, p1, fastbond()).expect("and so is the opponent's");
    assert_eq!(
        pt(&engine, atog),
        (1, 2),
        "a printed 1/2 before anything is eaten"
    );

    // The card's only ability, and its price is a permanent rather than mana,
    // so the question arrives the moment it is pressed.
    activate(&mut engine, p0, auratog(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which enchantment, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one enchantment, no more and no fewer");
    assert_eq!(
        options,
        vec![fodder],
        "the enchantment you control is the whole of the menu: not the \
         opponent's (CR 701.21a) and not the Sol Ring standing beside it"
    );
    assert!(
        !options.contains(&theirs) && !options.contains(&atog),
        "an opponent's enchantment is not yours to sacrifice and a creature \
         is no enchantment: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the enchantment the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, atog),
        (3, 4),
        "\"gets +2/+2 until end of turn\" on the 1/2 it was printed as"
    );
    assert!(
        in_graveyard(&engine, p0, fastbond()).is_some(),
        "the sacrificed enchantment is in the graveyard of the seat that paid it"
    );
    assert!(
        on_battlefield(&engine, p1, fastbond()).is_some(),
        "and the enchantment that paid nothing never moved"
    );
}
