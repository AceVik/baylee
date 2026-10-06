//! `cards/creatures/mv_4/subtlety.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When this creature enters, choose up to one target creature spell or
/// planeswalker spell. Its owner puts it on their choice of the top or
/// bottom of their library." Flash lets seat 0 cast it on the opponent's
/// turn in answer to Steadfast Guard; the opponent, who owns the Guard, is
/// the one asked, and takes the top.
#[test]
fn subtlety_lets_the_owner_put_the_spell_on_top() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, guard) = subtlety_answers_a_guard(
        &[island(), island(), island(), island()],
        &[subtlety()],
        false,
    );
    let Pending::YesNo { player, prompt, .. } = subtlety_sends(&mut engine, guard) else {
        unreachable!()
    };
    assert_eq!(
        player, p1,
        "the spell's owner chooses, not Subtlety's controller"
    );
    assert!(matches!(
        prompt,
        crate::choice::YesNoPrompt::TopOfLibrary { card } if card == guard
    ));
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(steadfast_guard()),
        "the Guard is the top card of its owner's library"
    );
    assert!(on_battlefield(&engine, p1, steadfast_guard()).is_none());
    let subtlety = on_battlefield(&engine, p0, subtlety()).expect("cast for its mana cost");
    assert!(keywords(&engine, subtlety).contains(KeywordSet::FLYING));
}

/// Evoked for a blue card from hand, Subtlety is sacrificed, and the owner's
/// "no" puts the Guard on the bottom.
#[test]
fn subtlety_evoked_and_the_owner_takes_the_bottom() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, guard) = subtlety_answers_a_guard(&[], &[subtlety(), brainstorm()], true);
    let Pending::YesNo { player, .. } = subtlety_sends(&mut engine, guard) else {
        unreachable!()
    };
    assert_eq!(player, p1);
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let bottom = engine.state().zones.list(ZoneLocation::Library(p1))[0];
    assert_eq!(
        engine.state().object(bottom).unwrap().card.map(|c| c.index),
        Some(steadfast_guard()),
        "the Guard is the bottom card of its owner's library"
    );
    assert!(on_battlefield(&engine, p0, subtlety()).is_none(), "evoked");
    assert!(in_graveyard(&engine, p0, subtlety()).is_some());
    assert!(
        in_hand(&engine, p0, brainstorm()).is_none(),
        "exiled to pay"
    );
}
