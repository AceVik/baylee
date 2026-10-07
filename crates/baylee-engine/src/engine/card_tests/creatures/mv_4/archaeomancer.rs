//! `cards/creatures/mv_4/archaeomancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archaeomancer — {2}{U}{U}, a 1/2 Human Wizard: "When this creature enters,
/// return target instant or sorcery card from your graveyard to your hand."
///
/// The entry trigger is given three cards to choose between, and only one of
/// them answers both printed words at once: a Llanowar Elves in each graveyard
/// and the Dark Ritual that has just resolved into this seat's own. The
/// creature card beside it rules out "instant or sorcery", the same card
/// across the table rules out "your graveyard", and the card that survives
/// both filters is then read back in hand rather than merely named.
#[test]
fn archaeomancer_returns_only_an_instant_or_sorcery_from_your_own_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The filler deck is a creature card on purpose: `seed_graveyard` only
    // reaches a library, so a creature has to come from there for one to stand
    // beside the spell the Archaeomancer is meant to find.
    let mut engine = Duel::new(211, llanowar_elves())
        .battlefield(0, &[swamp(), island(), island(), island(), island()])
        .hand(0, &[archaeomancer(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = in_graveyard(&engine, p0, llanowar_elves()).expect("p0's graveyard was seeded");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("p1's graveyard was seeded");

    // The one card that does qualify arrives the way a card arrives: cast off
    // the Swamp, resolved, and in its owner's graveyard.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    let ritual = in_graveyard(&engine, p0, dark_ritual()).expect("Dark Ritual resolved");

    cast_from_hand(&mut engine, p0, archaeomancer());
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the controller of the entering Wizard picks the card"
    );
    assert_eq!((min, max), (1, 1), "one card, and the trigger asks once");
    assert!(
        options.contains(&ritual),
        "the instant in this seat's own graveyard is the whole of the sentence: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is alone: the creature card beside it is no instant or sorcery, \
         and the one across the table is not this seat's: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "\"instant or sorcery\" declines the Llanowar Elves in the same graveyard: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"from your graveyard\" declines the same card across the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, dark_ritual()).is_some(),
        "\"return target instant or sorcery card … to your hand\""
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_none(),
        "the card left the graveyard rather than being copied out of it"
    );
    assert!(
        on_battlefield(&engine, p0, archaeomancer()).is_some(),
        "the Wizard itself stays on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some()
            && in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and neither card the trigger declined moved"
    );
}
