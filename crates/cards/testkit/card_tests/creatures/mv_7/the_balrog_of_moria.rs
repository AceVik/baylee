//! `cards/creatures/mv_7/the_balrog_of_moria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Declined: the Balrog stays in the graveyard, nothing triggers and nobody
/// is asked for a target.
#[test]
fn the_balrog_of_moria_left_in_the_graveyard_exiles_nothing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, _) = balrog_dies(&[&[thundering_giant()]]);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(in_graveyard(&engine, p0, the_balrog_of_moria()).is_some());
    assert!(on_battlefield(&engine, p1, thundering_giant()).is_some());
}

/// "Cycling {3}{R}" and "When you cycle this card, create two Treasure
/// tokens": the card is discarded as the cost, a card is drawn, and the
/// trigger fires from the graveyard (CR 702.29c).
#[test]
fn the_balrog_of_moria_cycled_draws_and_makes_two_treasures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[the_balrog_of_moria()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, the_balrog_of_moria(), 1);
    assert!(in_graveyard(&engine, p0, the_balrog_of_moria()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 2, "two Treasures");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "one card discarded, one drawn"
    );
}

/// Discarded for another reason — here to hand size in the cleanup step —
/// the Balrog is not cycled, and no Treasure is made.
#[test]
fn the_balrog_of_moria_discarded_to_hand_size_is_not_cycled() {
    let p0 = PlayerId::new(0);
    let mut hand = vec![the_balrog_of_moria()];
    hand.extend_from_slice(&[plains(); 8]);
    let mut engine = Duel::new(SEED, plains()).hand(0, &hand).start();
    keep_mulligans(&mut engine);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. }) || e.state().turn.number > turn
    });
    assert!(matches!(engine.pending(), Pending::DiscardChoice { .. }));
    let balrog = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == the_balrog_of_moria()))
        })
        .unwrap();
    let Pending::DiscardChoice { count, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let mut objects = vec![balrog];
    objects.extend(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .filter(|&id| id != balrog)
            .take(usize::from(count) - 1),
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects })
        .unwrap();
    assert!(in_graveyard(&engine, p0, the_balrog_of_moria()).is_some());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().turn.number > turn
    });
    assert!(tokens_of(&engine, p0).is_empty(), "no Treasure");
}
