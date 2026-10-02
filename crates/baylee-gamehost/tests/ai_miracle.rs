//! Miracle paid by the house through real engine offers and projected debt.
use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::DeckEntry;
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_engine::zone::Zone;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

#[test]
fn temporal_mastery_is_cast_for_miracle_with_no_floating_mana() {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Temporal Mastery").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = vec![entry("Island"); 3];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let me = PlayerId::new(0);
    let mut miracle = None;
    let mut taps = 0;
    for seq in 0..400 {
        if let Some(card) = miracle
            && engine
                .state()
                .object(card)
                .is_some_and(|o| o.zone == Zone::Exile)
        {
            assert!(
                !engine.state().extra_turns.is_empty(),
                "the extra turn was earned"
            );
            assert_eq!(taps, 2, "exactly {{1}}{{U}} was made in the payment window");
            assert_eq!(engine.state().players[0].mana_pool.total(), 0);
            return;
        }
        let pending = engine.pending();
        let seat = pending_player(pending).expect("game remains live");
        let context = SeatContext {
            awaiting: Some(seat),
            owed: owed_payment(&engine),
            ..Default::default()
        };
        let view = player_view(engine.state(), seat, seq, Some(pending), &context, &[]);
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::YesNo { .. } if seat != me => PlayerAction::YesNo(false),
            Pending::Priority { .. } if seat != me => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if let Pending::YesNo {
            prompt: YesNoPrompt::Miracle { card },
            ..
        } = pending
            && seat == me
        {
            assert_eq!(engine.state().players[0].mana_pool.total(), 0);
            assert_eq!(action, PlayerAction::YesNo(true));
            miracle = Some(*card);
        }
        if matches!(action, PlayerAction::ActivateManaAbility { .. }) && seat == me {
            assert_eq!(
                engine.payment_window(),
                Some((
                    me,
                    baylee_core::mana::ManaPayment::Fixed("{1}{U}".parse().unwrap())
                ))
            );
            taps += 1;
        }
        engine
            .apply(seat, action)
            .expect("every AI action is legal");
    }
    panic!("the AI never resolved Temporal Mastery through miracle");
}
