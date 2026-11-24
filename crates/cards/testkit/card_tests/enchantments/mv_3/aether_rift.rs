//! `cards/enchantments/mv_3/aether_rift.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aether_rift() -> CardIndex {
    card_index("2577ef8e-d85e-47af-ac34-629f31992751")
}

fn rift_with_hand(players: usize, card: CardIndex) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, island(), players)
        .battlefield(0, &[aether_rift()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    engine
}

#[track_caller]
fn pass_to_life_question(engine: &mut Engine<RegistryLookup>) -> PlayerId {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayLife { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate stopped at a life-payment question")
    };
    assert_eq!(
        prompt,
        YesNoPrompt::PayLife {
            amount: 5,
            permanent: None,
        }
    );
    player
}

#[test]
fn aether_rift_returns_a_discarded_creature_when_nobody_pays() {
    let mut engine = rift_with_hand(2, index::GRIZZLY_BEARS);

    assert_eq!(pass_to_life_question(&mut engine), P0);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(pass_to_life_question(&mut engine), P1);
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "the discarded creature returned under its owner's control"
    );
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn aether_rift_asks_in_apnap_order_and_stops_after_the_first_payment() {
    let mut engine = rift_with_hand(4, index::GRIZZLY_BEARS);

    assert_eq!(pass_to_life_question(&mut engine), P0);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(pass_to_life_question(&mut engine), P1);
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "one payment keeps the creature in the graveyard"
    );
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(engine.state().players[2].life, 20);
    assert_eq!(engine.state().players[3].life, 20);
    for seat in [PlayerId::new(2), PlayerId::new(3)] {
        assert!(
            engine.pending_for(seat).is_none(),
            "{seat:?} is not asked after seat 1 pays"
        );
    }
}

#[test]
fn aether_rift_discards_a_noncreature_without_asking_for_life() {
    let mut engine = rift_with_hand(2, island());

    reach_main_phase(&mut engine, P0);

    assert!(in_graveyard(&engine, P0, island()).is_some());
    assert!(!matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayLife { .. },
            ..
        }
    ));
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
