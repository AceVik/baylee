//! `cards/creatures/mv_2/brood_of_cockroaches.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn brood_of_cockroaches() -> CardIndex {
    card_index("7a22d694-29d1-4e7b-8fca-d7008aede489")
}

/// Brood of Cockroaches (`{1}{B}`, `Coverage::Implemented`): "When this
/// creature is put into your graveyard from the battlefield, at the beginning
/// of the next end step, you lose 1 life and return this card to your hand."
/// Right after it dies the card lies in the graveyard and nothing has been
/// lost; at that turn's end step the 1 life goes and the card is in the hand,
/// once and only once.
#[test]
fn brood_of_cockroaches_returns_to_hand_and_costs_1_life_at_the_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[brood_of_cockroaches()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, brood_of_cockroaches());
    pass_until(&mut engine, stack_is_empty);
    let brood = on_battlefield(&engine, p0, brood_of_cockroaches()).unwrap();
    assert_eq!(pt(&engine, brood), (1, 1));
    assert!(in_hand(&engine, p0, brood_of_cockroaches()).is_none());

    kill(&mut engine, brood);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, brood_of_cockroaches()).is_some(),
        "not returned yet: that waits for the end step"
    );
    assert!(in_hand(&engine, p0, brood_of_cockroaches()).is_none());
    assert_eq!(engine.state().players[0].life, 20, "no life lost yet");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "1 life lost at the end step, once"
    );
    assert!(
        in_hand(&engine, p0, brood_of_cockroaches()).is_some(),
        "returned to its owner's hand"
    );
    assert!(in_graveyard(&engine, p0, brood_of_cockroaches()).is_none());

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the delay fires only once"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

/// "Into your graveyard" excludes a stolen Brood: it goes to its owner's
/// graveyard, not the graveyard of the player who controlled it as it died.
#[test]
fn brood_of_cockroaches_does_not_trigger_when_its_controller_is_not_its_owner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[brood_of_cockroaches()])
        .hand(0, &[baylee_core::generated::index::CONTROL_MAGIC])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let brood = on_battlefield(&engine, p1, brood_of_cockroaches()).unwrap();
    cast_from_hand(
        &mut engine,
        p0,
        baylee_core::generated::index::CONTROL_MAGIC,
    );
    aim_at(&mut engine, p0, brood);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(brood).unwrap().controller, p0);

    kill(&mut engine, brood);
    assert!(in_graveyard(&engine, p1, brood_of_cockroaches()).is_some());
    reach_their_main_phase(&mut engine, p1);
    assert!(in_graveyard(&engine, p1, brood_of_cockroaches()).is_some());
    assert!(in_hand(&engine, p1, brood_of_cockroaches()).is_none());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
