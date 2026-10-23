//! `cards/instants/mv_2/sirocco.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;

fn sirocco() -> CardIndex {
    card_index("a9bdf857-0c6b-4afa-afbb-aff7e4b61b09")
}

/// Sirocco (`{1}{R}`, `Coverage::Partial`): target player reveals their hand.
/// The per-card "discards that card unless they pay 4 life" is not played, so
/// a forced discard that would take away the choice to pay 4 life is not
/// there either: the blue instant stays in hand and no life is paid.
#[test]
fn sirocco_reveals_the_targets_hand_and_discards_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[sirocco()])
        .hand(1, &[counterspell(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
    assert_eq!(theirs.len(), 2);

    cast_from_hand(&mut engine, p0, sirocco());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine.state().journal.entries().iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { player, cards } if *player == p1 && cards == &theirs
        )),
        "the targeted player's whole hand was revealed"
    );
    assert!(
        in_hand(&engine, p1, counterspell()).is_some(),
        "NOT SUPPORTED: the per-card discard-or-pay-4-life is not played, so the blue instant stays"
    );
    assert!(in_hand(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p1, counterspell()).is_none());
    assert_eq!(engine.state().players[1].life, 20, "no life was paid");
}

/// The reveal follows the target: aimed at the caster, it is the caster's own
/// hand that is shown and the opponent's stays hidden.
#[test]
fn sirocco_aimed_at_the_caster_reveals_only_the_casters_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[sirocco(), grizzly_bears()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sirocco());
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let revealed = |seat: PlayerId| {
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|e| matches!(&e.event, GameEvent::Revealed { player, .. } if *player == seat))
    };
    assert!(revealed(p0), "the caster named themselves");
    assert!(!revealed(p1), "the opponent's hand was not the target");
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(in_hand(&engine, p1, counterspell()).is_some());
}
