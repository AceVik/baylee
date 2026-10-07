//! `cards/creatures/mv_4/goblin_settler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Settler is `{3}{R}` for a 1/1 body and one printed sentence: "When
/// this creature enters, destroy target land." The trigger is the card, so the
/// board carries a land on either side of the table and a creature beside
/// them: `Filter::LAND` names no side, so both seats' lands are on the menu,
/// while the Elf is what says the filter is read rather than skipped. The four
/// Mountains are exactly the mana cost and stay standing afterwards, so the
/// land that goes to a graveyard can only be the one the trigger named.
#[test]
fn goblin_settler_destroys_a_land_of_either_seat_and_never_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[goblin_settler()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, mountain()).expect("my Mountain is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    cast_from_hand(&mut engine, p0, goblin_settler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature and no land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Forest was one of the options the trigger enumerated");
    pass_until(&mut engine, stack_is_empty);

    let settler = on_battlefield(&engine, p0, goblin_settler()).expect("the Settler landed");
    assert_eq!(pt(&engine, settler), (1, 1), "the body the card prints");
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "the targeted land was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and left the battlefield, which is what destroy means"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_some(),
        "the land the trigger did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature standing beside it"
    );
}
