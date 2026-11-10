//! `cards/instants/mv_1/silence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silence: "Your opponents can't cast spells this turn."
///
/// Two moments, because a lock that was already on would read the same as
/// one that arrived: while Silence is still on the stack the opponent may
/// answer it, and once it has resolved the same card in the same hand is
/// gone from the offer.
///
/// The opponent's spell is a **Brainstorm** and the choice is the whole
/// test. The first draft gave them a Counterspell, which passed against an
/// engine with this rule removed: with Silence resolved there is nothing on
/// the stack, so a counter is refused for having no target and the negative
/// was true of a game that had never heard of Silence. Brainstorm names
/// nothing, so the only thing that can refuse it is the lock — and the
/// `{U}{U}` it would be paid with is asserted to be still floating, because
/// an offer is gated on affordability too.
#[test]
fn silence_takes_an_opponents_spells_away_once_it_has_resolved_and_not_before() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .battlefield(1, &[island(), island()])
        .hand(0, &[silence()])
        .hand(1, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, silence());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);

    let storm = in_hand(&engine, p1, brainstorm()).expect("the Brainstorm is in hand");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert!(
        legal.castable.contains(&storm),
        "Silence is on the stack and has not resolved: its own controller \
         may still be answered"
    );

    // Let it resolve, then come back to p1 with the stack empty.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && e.state().zones.stack_is_empty()
    });
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "the {{U}}{{U}} is still floating, so the price is not what refuses \
         the Brainstorm below"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&storm),
        "\"your opponents can't cast spells this turn\""
    );
    assert!(
        legal.castable.is_empty(),
        "and not this one spell in particular: {:?}",
        legal.castable
    );
}
