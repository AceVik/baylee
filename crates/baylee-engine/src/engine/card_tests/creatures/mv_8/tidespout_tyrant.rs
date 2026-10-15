//! `cards/creatures/mv_8/tidespout_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tidespout_tyrant() -> CardIndex {
    card_index("331b44e4-cf08-44f4-8afd-a7374e5dc5ed")
}

/// Tidespout Tyrant is a 5/5 flier for {5}{U}{U}{U}.
#[test]
fn tidespout_tyrant_is_cast_for_eight_with_flying() {
    let mut engine = at_main(&[island(); 8], &[tidespout_tyrant()]);
    let djinn = cast_and_settle(&mut engine, tidespout_tyrant());
    assert_eq!(pt(&engine, djinn), (5, 5));
    assert!(keywords(&engine, djinn).contains(KeywordSet::FLYING));
}

/// "Whenever you cast a spell, return target permanent to its owner's hand":
/// casting a Bears names the opponent's Elves, which go back to their owner's
/// hand while the spell is still on the stack.
#[test]
fn tidespout_tyrant_bounces_a_permanent_when_its_controller_casts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tidespout_tyrant(), forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    assert!(in_hand(&engine, p1, llanowar_elves()).is_none());

    cast_from_hand(&mut engine, p0, grizzly_bears());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&elves), "any permanent: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "back in its owner's hand"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "the spell resolved too"
    );
}

/// The opponent's spells do not trigger it.
#[test]
fn tidespout_tyrant_ignores_the_opponents_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tidespout_tyrant()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, p0, tidespout_tyrant()).is_some(),
        "nothing was bounced"
    );
}
