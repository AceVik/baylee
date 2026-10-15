//! `cards/creatures/artifacts/mv_4/molten_tail_masticore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

fn molten_tail_masticore() -> CardIndex {
    card_index("15fb3a7a-a6c3-4bef-9f72-1d04b623063e")
}

/// Passes priority to the upkeep trigger's "discard a card, or sacrifice"
/// question and returns the cards on offer.
#[track_caller]
fn upkeep_discard_options(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                options,
                prompt: ChoicePrompt::CostDiscard,
                ..
            } => return options,
            other => panic!("unexpected before the upkeep question: {other:?}"),
        }
    }
    panic!("the upkeep never asked");
}

/// Seat 0's Molten-Tail Masticore through its first upkeep, paying the
/// discard with `hand`'s one card, and on into the first main phase with
/// four Forests to spend. The graveyard holds what was discarded.
#[track_caller]
fn in_the_main_phase_having_discarded(discarded: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                molten_tail_masticore(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[discarded])
        .start();
    keep_mulligans(&mut engine);
    let options = upkeep_discard_options(&mut engine);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("pay the discard");
    answer_until_settled(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    engine
}

/// Molten-Tail Masticore — {4} 4/4: "At the beginning of your upkeep,
/// sacrifice this creature unless you discard a card." Declining the
/// discard sacrifices it.
#[test]
fn molten_tail_masticore_is_sacrificed_when_the_discard_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[molten_tail_masticore()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    upkeep_discard_options(&mut engine);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("discarding nothing is a legal answer: the cost is optional");
    answer_until_settled(&mut engine);
    assert!(in_graveyard(&engine, p0, molten_tail_masticore()).is_some());
    assert!(on_battlefield(&engine, p0, molten_tail_masticore()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
}

/// "{4}, Exile a creature card from your graveyard: This creature deals 4
/// damage to any target." The Bears discarded at the upkeep are the creature
/// card; exiled, they leave the graveyard and the opponent takes 4.
#[test]
fn molten_tail_masticore_exiles_a_creature_card_to_deal_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = in_the_main_phase_having_discarded(grizzly_bears());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    activate(&mut engine, p0, molten_tail_masticore(), 1);
    // The exile may ask which card, and the target which player.
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("a player is a legal target");
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("the Bears are on the menu");
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 16, "four damage");
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_none(),
        "exiled as a cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{4}} paid");
}

/// The cost needs a *creature* card: with only a Sol Ring in the graveyard
/// the ability is not offered, four mana or not.
#[test]
fn molten_tail_masticore_needs_a_creature_card_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = in_the_main_phase_having_discarded(sol_ring());
    assert!(in_graveyard(&engine, p0, sol_ring()).is_some());
    let beast = on_battlefield(&engine, p0, molten_tail_masticore()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(beast, 1)));
}

/// "{2}: Regenerate this creature." Bought, a destroy leaves it standing,
/// tapped.
#[test]
fn molten_tail_masticore_regenerates() {
    let p0 = PlayerId::new(0);
    let mut engine = in_the_main_phase_having_discarded(sol_ring());
    let beast = on_battlefield(&engine, p0, molten_tail_masticore()).expect("seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, beast, 2);
    kill(&mut engine, beast);
    assert!(on_battlefield(&engine, p0, molten_tail_masticore()).is_some());
    assert!(is_tapped(&engine, beast));
}
