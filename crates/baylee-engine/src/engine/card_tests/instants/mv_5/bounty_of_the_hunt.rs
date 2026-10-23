//! `cards/instants/mv_5/bounty_of_the_hunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bounty_of_the_hunt() -> CardIndex {
    card_index("77a43c24-398f-42c6-a0d3-b28b33a0ce12")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Casts the Bounty from `p0`'s hand at `target`, answering what is asked: the cast mode (the
/// alternative cost when `alternative`, the printed one otherwise) and the card to exile for it.
/// Returns the cards the exile question offered (empty when none was asked).
fn cast_bounty(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    target: ObjectId,
    alternative: bool,
    pitch: ObjectId,
) -> Vec<ObjectId> {
    let bounty = in_hand(engine, p0, bounty_of_the_hunt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bounty })
        .unwrap();
    let mut offered = Vec::new();
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| {
                        if alternative {
                            matches!(o.kind, CastModeKind::Alternative(_))
                        } else {
                            matches!(o.kind, CastModeKind::Normal)
                        }
                    })
                    .expect("the cast mode is on offer");
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![target],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                offered = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending during the cast: {other:?}"),
        }
    }
    offered
}

/// `Bounty of the Hunt` ({3}{G}{G}, Instant, `Coverage::Partial`): "You may exile a green card from
/// your hand rather than pay this spell's mana cost." Only a green card is offered, only that one
/// is exiled, and the target gets +3/+3 (no counters are put on it) until the turn ends.
#[test]
fn bounty_of_the_hunt_pitches_a_green_card_for_plus_three_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[hill_giant()])
        .hand(
            0,
            &[bounty_of_the_hunt(), grizzly_bears(), lightning_bolt()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let their_giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let pitch = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    let offered = cast_bounty(&mut engine, p0, giant, true, pitch);
    assert!(offered.contains(&pitch), "a green card is offered");
    assert!(!offered.contains(&bolt), "a red card is not");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(pitch).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(bolt).unwrap().zone,
        Zone::Hand,
        "only the chosen card is exiled"
    );
    assert_eq!(pt(&engine, giant), (6, 6));
    assert_eq!(pt(&engine, their_giant), (3, 3), "only the target");

    // It ends with the turn.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(pt(&engine, giant), (3, 3));
}

/// "May": paying the printed mana cost exiles nothing.
#[test]
fn bounty_of_the_hunt_paid_with_mana_keeps_the_green_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                hill_giant(),
            ],
        )
        .hand(0, &[bounty_of_the_hunt(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let kept = in_hand(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_bounty(&mut engine, p0, giant, false, kept);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(kept).unwrap().zone, Zone::Hand);
    assert_eq!(pt(&engine, giant), (6, 6));
}

/// Without a green card to exile and without the mana, the Bounty cannot be cast.
#[test]
fn bounty_of_the_hunt_cannot_be_cast_for_free_without_a_green_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .hand(0, &[bounty_of_the_hunt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bounty = in_hand(&engine, p0, bounty_of_the_hunt()).unwrap();
    let result = engine.apply(p0, PlayerAction::CastSpell { card: bounty });
    let gets_free_cast = match result {
        Err(_) => false,
        Ok(()) => matches!(
            engine.pending(),
            Pending::ChooseCastMode { options, .. }
                if options.iter().any(|o| matches!(o.kind, CastModeKind::Alternative(_)))
        ),
    };
    assert!(!gets_free_cast, "a red card is not a green card");
}
