//! `cards/creatures/mv_3/ghastly_remains.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;
use baylee_engine::choice::{ChoicePrompt, YesNoPrompt};
use baylee_engine::event::GameEvent;
use baylee_engine::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ghastly_remains() -> CardIndex {
    card_index("0eafe734-19f2-492d-bfc2-d0c75ece45b2")
}

/// Ghastly Remains ({B}{B}{B}, 0/0 Zombie):
/// Amplify 1; At the beginning of your upkeep, if this card is in your
/// graveyard, you may pay {B}{B}{B}. If you do, return it to your hand.
#[test]
fn ghastly_remains_survives_when_amplified_and_stays_silent_on_battlefield_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[ghastly_remains(), scathe_zombies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, ghastly_remains());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 1);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let remains = on_battlefield(&engine, P0, ghastly_remains())
        .expect("0/0 Ghastly Remains survives with +1/+1 counter from Amplify");
    assert_eq!(counters_on(&engine, remains, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, remains), (1, 1));

    // Advance to P0's next upkeep; intervening-if requires graveyard, so no trigger fires.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, ghastly_remains()).is_some());
    assert!(in_graveyard(&engine, P0, ghastly_remains()).is_none());
}

#[test]
fn ghastly_remains_dies_at_zero_zero_skips_opponents_upkeep_and_returns_when_paid() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[ghastly_remains(), scathe_zombies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Cast and decline to reveal Zombie: enters 0/0, SBA sends it to graveyard.
    cast_from_hand(&mut engine, P0, ghastly_remains());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, ghastly_remains()).is_none());
    assert!(in_graveyard(&engine, P0, ghastly_remains()).is_some());

    // Opponent's upkeep must not trigger Ghastly Remains.
    let seq_before_opp = engine.journal().last_seq();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Main && stack_is_empty(e)
    });
    let triggered_on_opp = engine
        .journal()
        .entries()
        .iter()
        .skip(usize::try_from(seq_before_opp).unwrap())
        .any(|e| matches!(e.event, GameEvent::AbilityTriggered { .. }));
    assert!(
        !triggered_on_opp,
        "Ghastly Remains must not trigger on opponent's upkeep"
    );
    assert!(in_graveyard(&engine, P0, ghastly_remains()).is_some());

    // Turn 3 (P0's upkeep): decline paying {B}{B}{B} -> remains in graveyard.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. } | YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(engine.state().turn.active, P0);
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    if let Pending::YesNo {
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    {
        assert_eq!(*cost, baylee_core::mana!("{B}{B}{B}"));
    }
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, ghastly_remains()).is_some());
    assert!(in_hand(&engine, P0, ghastly_remains()).is_none());
    assert!(
        all_on_battlefield(&engine, P0, swamp())
            .iter()
            .all(|&id| !is_tapped(&engine, id)),
        "declining upkeep payment leaves all Swamps untapped"
    );

    // Turn 5 (P0's next upkeep): accept paying {B}{B}{B}, tap 3 Swamps in payment window, return to hand.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 5
            && e.state().turn.step == Step::Upkeep
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            )
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        engine.payment_window().is_some(),
        "accepting {{B}}{{B}}{{B}} opens mana payment window"
    );
    let swamps = all_on_battlefield(&engine, P0, swamp());
    tap_mana_where(&mut engine, P0, |id| swamps.contains(&id));
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        swamps.iter().all(|&id| is_tapped(&engine, id)),
        "paying {{B}}{{B}}{{B}} tapped all 3 Swamps"
    );
    assert!(in_hand(&engine, P0, ghastly_remains()).is_some());
    assert!(in_graveyard(&engine, P0, ghastly_remains()).is_none());
}

#[test]
fn ghastly_remains_accepts_changeling_in_hand_for_amplify() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[ghastly_remains(), index::CRIB_SWAP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, ghastly_remains());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let changeling = in_hand(&engine, P0, index::CRIB_SWAP).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&changeling));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![changeling],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let remains = on_battlefield(&engine, P0, ghastly_remains()).unwrap();
    assert_eq!(pt(&engine, remains), (1, 1));
}
