//! `cards/creatures/mv_5/solitude.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The same sentence on a creature, and the reason the sweep named two cards
/// and not one: Solitude exiles and reads through a *trigger* rather than a
/// spell, which is a second resolution path to the same `Amount`.
#[test]
fn solitudes_trigger_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(42, solitude(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let incarnation = in_hand(&engine, p0, solitude()).expect("Solitude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: incarnation })
        .unwrap();
    // Solitude itself targets nothing; the first target question belongs to
    // its enters trigger, and the Raptor is the only other creature.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

/// Solitude: "Evoke—Exile a white card from your hand." and "When this
/// creature enters, exile up to one other target creature. That creature's
/// controller gains life equal to its power." An evoked Solitude is
/// sacrificed once it has entered; its trigger still exiles.
#[test]
fn solitude_evoked_by_pitching_a_white_card_exiles_its_target_and_is_sacrificed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let reanimate = card_index("a044474a-cd72-4e9d-bd8d-a08f2de9cdc0");
    let mut engine = Duel::new(4502, island())
        .hand(0, &[solitude(), savannah_lions(), reanimate])
        .battlefield(0, &[swamp()])
        .battlefield(1, &[grizzly_bears()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears");
    let incarnation = in_hand(&engine, p0, solitude()).expect("Solitude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: incarnation })
        .expect("Solitude is castable with no mana, by pitching");
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let alt = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the evoke cost is offered");
                engine.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                let lions = options
                    .iter()
                    .copied()
                    .find(|o| engine_object_is(&engine, *o, savannah_lions()))
                    .expect("the white card is the only one offered");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![lions],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("the Bears are the only other creature");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        cards_of_in(&engine, ZoneLocation::Exile(p1), grizzly_bears()),
        1,
        "exiled"
    );
    assert_eq!(
        engine.state().players[1].life,
        22,
        "their controller gains life equal to its power"
    );
    assert_eq!(
        cards_of_in(&engine, ZoneLocation::Exile(p0), savannah_lions()),
        1,
        "the white card was the price"
    );
    assert!(
        on_battlefield(&engine, p0, solitude()).is_none(),
        "evoked: sacrificed as it entered"
    );
    assert!(in_graveyard(&engine, p0, solitude()).is_some());
    assert_reanimated_solitude_stays(&mut engine, p0, reanimate);
}
