use super::*;
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::PlayerAction;
use baylee_view::{PlayerView, SeatIdentity};
use prost::Message as _;
use std::cell::RefCell;
use std::rc::Rc;

const ME: PlayerId = PlayerId::new(0);

#[derive(Default)]
struct Calls {
    initialized: usize,
    observations: Vec<i32>,
    answers: usize,
    fail_observation: bool,
}

struct Brain(Rc<RefCell<Calls>>);
impl Brain4 for Brain {
    fn initialize(
        &mut self,
        seat: PlayerId,
        deck: &DeckList,
        teams: &[Option<u8>],
    ) -> anyhow::Result<()> {
        assert_eq!(seat, ME);
        assert_eq!(deck.name, "Own deck only");
        assert_eq!(teams, &[None, None]);
        self.0.borrow_mut().initialized += 1;
        Ok(())
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        assert!(view.casting.is_none() && view.targeting.is_none());
        let mut calls = self.0.borrow_mut();
        anyhow::ensure!(!calls.fail_observation, "mock observation failed");
        calls.observations.push(view.seats[0].life);
        Ok(())
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        _: &[Option<u8>],
    ) -> anyhow::Result<LiveAnswer4> {
        assert!(view.casting.is_none() && view.targeting.is_none());
        let mut calls = self.0.borrow_mut();
        assert_eq!(calls.observations.last(), Some(&view.seats[0].life));
        calls.answers += 1;
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::MulliganBottom { count, .. } => PlayerAction::ChooseObjects {
                objects: (0..u32::from(*count))
                    .map(|index| baylee_core::ids::ObjectId::new(index, 0))
                    .collect(),
            },
            _ => PlayerAction::ChooseColor(if view.seats[0].life < 20 {
                ManaColor::Red
            } else {
                ManaColor::Green
            }),
        };
        Ok(LiveAnswer4 {
            action,
            source: SourceKind4::Net,
        })
    }
}

fn adapter() -> (LiveSeat4<Brain>, Rc<RefCell<Calls>>) {
    let calls = Rc::new(RefCell::new(Calls::default()));
    (
        LiveSeat4::new(
            Brain(calls.clone()),
            DeckList {
                name: "Own deck only".into(),
                ..DeckList::default()
            },
            ME,
        ),
        calls,
    )
}

fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
    Envelope { msg: Some(msg) }.encode_to_vec()
}

fn table() -> Vec<u8> {
    let mut game = statics(0);
    game.seats = (0..2)
        .map(|index| SeatIdentity {
            player: PlayerId::new(index),
            display_name: if index == 0 {
                "NN-v4-R02".into()
            } else {
                "Human".into()
            },
            is_ai: false,
            away: false,
            team: None,
        })
        .collect();
    frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        static_json: serde_json::to_vec(&game).unwrap(),
    }))
}

fn view(seq: u64, life: i32) -> Vec<u8> {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    view.seats[0].life = life;
    view.awaiting = Some(ME);
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    }))
}

fn question(seq: u64) -> Vec<u8> {
    let pending = Pending::ChooseColor {
        player: ME,
        options: vec![ManaColor::Red, ManaColor::Green],
    };
    frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
        game_id: "test-game".into(),
        seq,
        pending_json: serde_json::to_vec(&pending).unwrap(),
    }))
}

fn curtain() -> Vec<u8> {
    frame(v1::envelope::Msg::Curtain(v1::Curtain {}))
}

#[test]
fn observes_passive_changes_deduplicates_resends_and_never_resets_on_statics() {
    let (mut seat, calls) = adapter();
    assert!(seat.hear(&table()).unwrap().is_empty());
    let ready = seat.hear(&view(1, 20)).unwrap();
    assert!(matches!(
        ready[0].msg,
        Some(v1::envelope::Msg::SeatReady(_))
    ));
    seat.hear(&view(1, 20)).unwrap();
    seat.hear(&view(2, 19)).unwrap();
    seat.hear(&table()).unwrap();
    assert_eq!(calls.borrow().initialized, 1);
    assert_eq!(calls.borrow().observations, vec![20, 19]);
    assert_eq!(calls.borrow().answers, 0);
    assert_eq!(seat.report().observations, 2);
}

#[test]
fn queued_question_waits_for_curtain_and_uses_latest_observation_once() {
    let (mut seat, calls) = adapter();
    seat.hear(&table()).unwrap();
    seat.hear(&view(1, 20)).unwrap();
    assert!(seat.hear(&question(1)).unwrap().is_empty());
    assert_eq!(calls.borrow().answers, 0);
    let outgoing = seat.hear(&curtain()).unwrap();
    let Some(v1::envelope::Msg::PlayerAction(action)) = &outgoing[0].msg else {
        panic!("action")
    };
    let answer: PlayerAction = serde_json::from_slice(&action.action_json).unwrap();
    assert_eq!(answer, PlayerAction::ChooseColor(ManaColor::Green));
    assert!(action.seat_token.is_empty());
    assert!(seat.hear(&question(1)).unwrap().is_empty());
    seat.hear(&view(2, 19)).unwrap();
    let outgoing = seat.hear(&question(2)).unwrap();
    let Some(v1::envelope::Msg::PlayerAction(action)) = &outgoing[0].msg else {
        panic!("action")
    };
    let answer: PlayerAction = serde_json::from_slice(&action.action_json).unwrap();
    assert_eq!(answer, PlayerAction::ChooseColor(ManaColor::Red));
    assert_eq!(calls.borrow().answers, 2);
    assert_eq!(calls.borrow().observations, vec![20, 19]);
    assert_eq!(seat.report().neural_answers, 2);
    assert_eq!(seat.report().duplicate_questions, 1);
}

#[test]
fn observation_failure_and_transport_gap_fail_closed() {
    let (mut seat, calls) = adapter();
    seat.hear(&table()).unwrap();
    seat.hear(&view(1, 20)).unwrap();
    seat.hear(&curtain()).unwrap();
    calls.borrow_mut().fail_observation = true;
    assert!(seat.hear(&view(2, 19)).is_err());
    assert!(seat.finished());
    assert!(seat.hear(&question(2)).unwrap().is_empty());
    assert_eq!(calls.borrow().answers, 0);
    let (mut other, _) = adapter();
    other.disconnected();
    assert!(other.finished());
    assert!(
        other
            .report()
            .failure
            .as_ref()
            .unwrap()
            .contains("interrupted")
    );
}

#[test]
fn question_before_its_view_waits_and_server_clock_echo_is_not_echoed() {
    let (mut seat, calls) = adapter();
    seat.hear(&table()).unwrap();
    seat.hear(&view(1, 20)).unwrap();
    seat.hear(&curtain()).unwrap();
    assert!(seat.hear(&question(2)).unwrap().is_empty());
    assert_eq!(calls.borrow().answers, 0);
    assert_eq!(seat.hear(&view(2, 19)).unwrap().len(), 1);
    let echo = frame(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
        client_time_ms: 17,
        server_time_ms: 23,
    }));
    assert!(seat.hear(&echo).unwrap().is_empty());
    assert_eq!(seat.report().clock_probes, 1);
}

#[test]
fn own_card_constructor_matches_offline_deck_features_without_opponent_decks() {
    use crate::features4::SeatKnowledge4;
    use baylee_core::preset::AIProfile;
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let offline = SeatKnowledge4::from_preset(&preset, ME);
    // This is the entire live constructor input: own counts and public size.
    let live = SeatKnowledge4::from_own_cards(ME, 2, &offline.own_deck_cards);
    assert_eq!(live.own_deck_cards, offline.own_deck_cards);
    assert_eq!(live.own_deck_summary, offline.own_deck_summary);
    assert_eq!(live.cursor, 0);
    assert!(live.known_cards.is_empty());
}

#[test]
fn simultaneous_opening_resends_at_later_seq_are_not_answered_twice() {
    for pending in [
        Pending::Mulligan {
            player: ME,
            taken: 0,
            next_is_free: true,
            can_take: true,
        },
        Pending::MulliganBottom {
            player: ME,
            count: 1,
        },
    ] {
        let (mut seat, calls) = adapter();
        seat.hear(&table()).unwrap();
        seat.hear(&view(1, 20)).unwrap();
        seat.hear(&curtain()).unwrap();
        let ask = |seq| {
            frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
                game_id: "test-game".into(),
                seq,
                pending_json: serde_json::to_vec(&pending).unwrap(),
            }))
        };
        assert_eq!(seat.hear(&ask(1)).unwrap().len(), 1);
        seat.hear(&view(2, 20)).unwrap();
        assert!(seat.hear(&ask(2)).unwrap().is_empty());
        assert_eq!(calls.borrow().answers, 1);
        assert_eq!(seat.report().duplicate_questions, 1);
    }
}

#[test]
fn changed_opening_count_and_normal_identical_question_at_later_seq_are_new_choices() {
    let (mut seat, calls) = adapter();
    seat.hear(&table()).unwrap();
    seat.hear(&view(1, 20)).unwrap();
    seat.hear(&curtain()).unwrap();
    for (seq, taken) in [(1, 0), (2, 1)] {
        seat.hear(&view(seq, 20)).unwrap();
        let pending = Pending::Mulligan {
            player: ME,
            taken,
            next_is_free: taken == 0,
            can_take: true,
        };
        let ask = frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
            game_id: "test-game".into(),
            seq,
            pending_json: serde_json::to_vec(&pending).unwrap(),
        }));
        assert_eq!(seat.hear(&ask).unwrap().len(), 1);
    }
    for seq in [3, 4] {
        seat.hear(&view(seq, 20)).unwrap();
        assert_eq!(seat.hear(&question(seq)).unwrap().len(), 1);
    }
    assert_eq!(calls.borrow().answers, 4);
}

#[test]
fn refusal_reports_known_safe_classes_and_omits_unknown_server_text() {
    for (message, expected) in [
        (
            "illegal action for your seat",
            "illegal action for your seat",
        ),
        ("not a human seat", "not a human seat"),
        (
            "private-card-name ticket=secret",
            "unclassified server refusal (text omitted)",
        ),
    ] {
        let (mut seat, _) = adapter();
        let error = frame(v1::envelope::Msg::Error(v1::Error {
            code: 400,
            message: message.into(),
        }));
        assert!(seat.hear(&error).is_err());
        let reason = seat.report().failure.as_ref().unwrap();
        assert!(reason.contains(expected));
        assert!(!reason.contains("private-card-name"));
        assert!(!reason.contains("ticket="));
    }
}

#[test]
fn socket_casting_and_targeting_are_normalized_for_policy_only() {
    use crate::features4::{encode_candidates4, encode_state4};
    use baylee_client_core::test_support::token;
    use baylee_engine::choice::LegalActions;
    let mut wire = ViewBuilder::new(2).build();
    wire.seq = 1;
    wire.awaiting = Some(ME);
    wire.casting = Some(baylee_core::ids::ObjectId::new(7, 0));
    wire.targeting = Some(baylee_view::TargetingContext {
        source: token(7, 0, "source", 2, 2),
        text: None,
        whole_spell: false,
        second: false,
        batch_count: 1,
    });
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    };
    let compatible = policy_view(&wire);
    let state = encode_state4(compatible.as_ref(), Some(&pending), None, &[None, None]);
    let choices = encode_candidates4(&state, compatible.as_ref(), &pending);
    assert!(
        choices
            .candidates
            .iter()
            .all(|choice| !matches!(choice.action, Some(PlayerAction::CancelCast)))
    );
    assert_eq!(choices.candidates.len(), 1);
    assert!(matches!(policy_view(compatible.as_ref()), Cow::Borrowed(_)));
    let (mut seat, calls) = adapter();
    seat.hear(&table()).unwrap();
    let delta = frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq: 1,
        view_json: serde_json::to_vec(&wire).unwrap(),
        log_json: Vec::new(),
    }));
    seat.hear(&delta).unwrap();
    seat.hear(&curtain()).unwrap();
    seat.hear(&question(1)).unwrap();
    assert_eq!(calls.borrow().answers, 1);
    assert_eq!(calls.borrow().observations.len(), 1);
    // UI-only changes must not advance recurrent memory, and TableMemory
    // continues to hold exactly the authorized wire view.
    wire.seq = 2;
    wire.targeting.as_mut().unwrap().batch_count = 2;
    seat.hear(&frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq: 2,
        view_json: serde_json::to_vec(&wire).unwrap(),
        log_json: Vec::new(),
    })))
    .unwrap();
    assert_eq!(calls.borrow().observations.len(), 1);
    let held = seat.memory.view().unwrap();
    assert_eq!(held.casting, wire.casting);
    assert_eq!(held.targeting.as_ref().unwrap().batch_count, 2);
}
