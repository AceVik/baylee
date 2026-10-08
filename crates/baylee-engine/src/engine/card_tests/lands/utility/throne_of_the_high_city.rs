//! `cards/lands/utility/throne_of_the_high_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Throne of the High City: "{4}, {T}, Sacrifice this land: You become the monarch."
/// Four Forests pay the generic cost to activate Throne of the High City.
/// The land is sacrificed as a cost, and upon resolution, its controller becomes the monarch.
#[test]
fn throne_of_the_high_city_sacrifices_to_make_controller_monarch() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(66, forest())
        .battlefield(
            0,
            &[
                throne_of_the_high_city(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().monarch, None);
    let throne = on_battlefield(&engine, p0, throne_of_the_high_city()).expect("Throne deployed");
    tap_mana_except(&mut engine, p0, throne);

    activate(&mut engine, p0, throne_of_the_high_city(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().monarch, Some(p0), "p0 became the monarch");
    assert!(on_battlefield(&engine, p0, throne_of_the_high_city()).is_none());
    assert!(in_graveyard(&engine, p0, throne_of_the_high_city()).is_some());
}

/// Passes priority until `seat` holds it.
#[track_caller]
fn pass_to(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected priority, got {other:?}"),
        }
    }
    panic!("{seat:?} never got priority");
}

/// The Throne is an activated ability, so any seat holding priority may use
/// it (CR 602.5d): seat 1 takes the crown during seat 0's turn, and the
/// monarch's draw is the beginning of *its own* end step (CR 724.2), not of
/// the active player's, which is the difference a table of four can show and
/// a duel cannot (with two seats "the other end step" is one seat).
#[test]
fn throne_of_the_high_city_crowns_a_non_active_seat_who_draws_at_its_own_end_step() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(6120, plains(), 4)
        .battlefield(
            1,
            &[
                throne_of_the_high_city(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    pass_to(&mut engine, p1);
    assert_eq!(
        engine.state().turn.active,
        p0,
        "it is seat 0's turn and seat 1 holds priority in it"
    );
    assert!(stack_is_empty(&engine));
    assert_eq!(engine.state().monarch, None);

    let throne = on_battlefield(&engine, p1, throne_of_the_high_city()).expect("the Throne");
    tap_mana_except(&mut engine, p1, throne);
    let from = engine.journal().entries().len();
    activate(&mut engine, p1, throne_of_the_high_city(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().monarch,
        Some(p1),
        "seat 1 became the monarch"
    );
    assert_eq!(
        engine.state().turn.active,
        p0,
        "and it happened on seat 0's turn"
    );
    let crowned: Vec<PlayerId> = engine.journal().entries()[from..]
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::BecameMonarch { player } => Some(player),
            _ => None,
        })
        .collect();
    assert_eq!(crowned, [p1], "once, and for the activating seat");
    assert!(
        in_graveyard(&engine, p1, throne_of_the_high_city()).is_some(),
        "the Throne was sacrificed as a cost"
    );

    let library = library_size(&engine, p1);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        library_size(&engine, p1),
        library - 1,
        "only seat 1's draw step so far: seat 0's end step drew nothing for the \
         monarch, because it is not the monarch's own"
    );
    reach_their_main_phase(&mut engine, p2);
    assert_eq!(
        library_size(&engine, p1),
        library - 2,
        "and seat 1's own end step drew a second card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand + 2,
        "into its hand"
    );
}

/// A monarch who activates the Throne becomes what they already are:
/// nothing is journaled (CR 724.3), the land is still spent, and the crown
/// stays where it was.
#[test]
fn throne_of_the_high_city_under_the_monarch_writes_no_crowning() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(6121, plains(), 4)
        .battlefield(
            0,
            &[
                throne_of_the_high_city(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
        .dev_state_mut(p0)
        .expect("the harness trusts itself")
        .set_monarch(p0);
    assert_eq!(engine.state().monarch, Some(p0));

    let throne = on_battlefield(&engine, p0, throne_of_the_high_city()).expect("the Throne");
    tap_mana_except(&mut engine, p0, throne);
    let from = engine.journal().entries().len();
    activate(&mut engine, p0, throne_of_the_high_city(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, throne_of_the_high_city()).is_some(),
        "the ability resolved: its cost sacrificed the land"
    );
    assert!(
        engine.journal().entries()[from..]
            .iter()
            .all(|e| !matches!(e.event, GameEvent::BecameMonarch { .. })),
        "the monarch told to become the monarch becomes nothing"
    );
    assert_eq!(engine.state().monarch, Some(p0));
}
