use super::*;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{NumberPrompt, timeout_answer};

#[test]
fn band_choice_bypasses_subset_sampling_without_claiming_a_policy_probability() {
    use baylee_core::ids::ObjectId;
    use baylee_engine::choice::ChoicePrompt;

    let pending = Pending::ChooseCards {
        player: PlayerId::new(0),
        options: (1..=3).map(|i| ObjectId::new(i, 0)).collect(),
        min: 0,
        max: 3,
        prompt: ChoicePrompt::Band {
            with: ObjectId::new(0, 0),
        },
        total: None,
    };
    for temperature in [0.0, 1.0] {
        let answer = fallback::unsupported_choice_answer(&pending, temperature).unwrap();
        assert_eq!(
            answer.action,
            PlayerAction::ChooseObjects {
                objects: Vec::new()
            }
        );
        assert!(pending.answer_fault(&answer.action).is_none());
        assert_eq!(answer.provenance.source, SourceKind4::House);
        assert!(!answer.provenance.has_logprob);
        assert_eq!(answer.provenance.behavior_logprob, 0.0);
        assert!(answer.target.multi_mask.is_empty());
        assert!(answer.target.free_mask.is_empty());
        assert!(answer.target.pick_sequence.is_empty());
    }
    let ordinary = Pending::ChooseNumber {
        player: PlayerId::new(0),
        min: 0,
        max: 3,
        reason: NumberPrompt::Choice,
    };
    assert!(fallback::unsupported_choice_answer(&ordinary, 1.0).is_none());
}

#[test]
fn optional_band_fallback_is_accepted_when_all_three_offered_members_are_not() {
    use baylee_core::ids::Defender;
    use baylee_engine::choice::ChoicePrompt;
    use baylee_engine::engine::Engine;

    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let template = preset.seats[0].deck[0];
    preset.seats[0].starting_battlefield =
        ["Benalish Hero", "Grizzly Bears", "Hill Giant", "Gray Ogre"]
            .into_iter()
            .map(|name| baylee_core::preset::DeckEntry {
                card: baylee_cards::decks::by_name(name).unwrap(),
                rule_print: None,
                ..template
            })
            .collect();
    let mut engine = Engine::new(&preset, baylee_gamehost::RegistryLookup).unwrap();
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let pending = engine.pending().clone();
        let player = pending.asked().unwrap();
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            // The engine asks every active player for a declaration, also
            // one with nothing to attack with.
            Pending::ChooseAttackers { .. } if player != me => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            // The opponent, who moves first, discards to hand size.
            Pending::DiscardChoice { count, .. } => PlayerAction::ChooseObjects {
                objects: engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Hand(player))
                    .iter()
                    .take(usize::from(count))
                    .copied()
                    .collect(),
            },
            Pending::ChooseAttackers { attackers, .. } => {
                assert_eq!(attackers.len(), 4);
                PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .into_iter()
                        .map(|id| (id, Defender::Player(PlayerId::new(1))))
                        .collect(),
                }
            }
            Pending::ChooseCards {
                ref options,
                prompt: ChoicePrompt::Band { .. },
                ..
            } => {
                assert_eq!(options.len(), 3);
                let invalid = PlayerAction::ChooseObjects {
                    objects: options.clone(),
                };
                assert!(pending.answer_fault(&invalid).is_none());
                assert!(engine.apply(player, invalid).is_err());
                let answer = fallback::unsupported_choice_answer(&pending, 1.0).unwrap();
                assert_eq!(answer.provenance.source, SourceKind4::House);
                assert!(!answer.provenance.has_logprob);
                engine.apply(player, answer.action).unwrap();
                return;
            }
            other => panic!("unexpected setup question: {other:?}"),
        };
        engine.apply(player, action).unwrap();
    }
    panic!("band question was not reached");
}

#[test]
fn missing_timeout_default_never_turns_a_number_question_into_a_pass() {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let session = baylee_gamehost::Session::new(&preset).unwrap();
    let view = session.observe_view(PlayerId::new(0));
    for (min, max, reason) in [(0, u32::MAX, NumberPrompt::X), (7, 9, NumberPrompt::Choice)] {
        let pending = Pending::ChooseNumber {
            player: view.seat,
            min,
            max,
            reason,
        };
        assert!(timeout_answer(&pending).is_none());
        let state = encode_state4(&view, Some(&pending), None, &[]);
        let cands = encode_candidates4(&state, &view, &pending);
        let action = fallback_action(&cands, &view, &pending, &[]);
        assert!(
            pending.answer_fault(&action).is_none(),
            "{pending:?}: {action:?}"
        );
        assert!(matches!(action, PlayerAction::ChooseNumber(n) if (min..=max).contains(&n)));
    }
}
