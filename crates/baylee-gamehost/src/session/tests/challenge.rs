use super::*;
use crate::record::{self, Line, Source};
use baylee_engine::choice::PriorityHold;

fn setup() -> GamePreset {
    let mut preset = test_preset();
    preset.house_rules.challenge_policy_seed = Some(9_001);
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.decision_timeout_secs = 0;
    preset.house_rules.reconnect_window_secs = 0;
    preset.house_rules.time_extension_votes = false;
    preset.seats[0].starting_life = Some(4);
    let entry = |name| DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("tested card"),
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    };
    preset.seats[1].controller = SeatController::Ai(AIProfile {
        foresight: 0,
        ..AIProfile::CASUAL
    });
    preset.seats[1].starting_battlefield = vec![entry("Mountain"); 4];
    preset.seats[1].starting_hand = Some(vec![
        entry("Shock"),
        entry("Lightning Bolt"),
        entry("Shock"),
        entry("Mountain"),
    ]);
    preset
}

fn recorded(preset: &GamePreset) -> Session {
    let mut session = Session::new_recorded(preset, baylee_build::short()).unwrap();
    session.describe(uuid::Uuid::now_v7().to_string(), vec![]);
    session.discard_frames();
    session
}

fn house_inputs(bytes: &[u8]) -> Vec<PlayerAction> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).unwrap() {
            Line::Input {
                by: Source::House,
                action,
                ..
            } => Some(action),
            _ => None,
        })
        .collect()
}

#[test]
fn fixed_challenge_survives_new_ids_ui_settings_and_process_resumption() {
    let preset = setup();
    let mut plain = recorded(&preset);
    let mut changed = recorded(&preset);
    let human = PlayerId::new(0);
    let player = HeuristicAgent::new(AIProfile::STEADY).with_seed(17);
    let mut original = Vec::new();
    let mut interrupted = Vec::new();
    let mut resumed = false;
    for step in 0..256 {
        plain.pump_at_most(32);
        changed.pump_at_most(32);
        // The engine's full hash also counts accepted setting inputs. Compare
        // the actual table here; each full hash is verified against its own replay below.
        assert_eq!(
            record::seen(plain.engine.state()),
            record::seen(changed.engine.state()),
            "step {step}"
        );
        original.extend(plain.take_record());
        interrupted.extend(changed.take_record());
        if matches!(plain.engine.pending(), Pending::GameOver(_)) {
            assert!(resumed, "the resumption was exercised before the end");
            assert!(house_inputs(&original).len() > 3);
            assert_eq!(house_inputs(&original), house_inputs(&interrupted));
            assert_eq!(
                record::replay(&original).unwrap().engine.snapshot_hash(),
                plain.snapshot_hash()
            );
            assert_eq!(
                record::replay(&interrupted).unwrap().engine.snapshot_hash(),
                changed.snapshot_hash()
            );
            return;
        }
        let Some((pending, view)) = plain.table().view_for(human) else {
            continue;
        };
        // Repeatedly choosing the already-active UI setting changes transport seq,
        // but must never change the policy's noise or restart its internal memory.
        changed
            .act_slice(
                human,
                PlayerAction::SetPriorityHold(PriorityHold::Always),
                0,
            )
            .unwrap();
        assert_eq!(view.seq, changed.table().view_for(human).unwrap().1.seq);
        let action = player.act(&view, &pending);
        plain.act_slice(human, action.clone(), 0).unwrap();
        changed.act_slice(human, action, 0).unwrap();
        interrupted.extend(changed.take_record());
        changed =
            Session::resume_recorded(&interrupted).expect("fixed opponent verifies on restart");
        changed.describe(uuid::Uuid::now_v7().to_string(), vec![]);
        changed.discard_frames();
        resumed = true;
    }
    panic!("bounded test game did not end");
}

#[test]
fn a_challenge_never_times_out_replaces_a_human_or_gives_away_its_policy() {
    let mut session = recorded(&setup());
    assert!(session.is_challenge());
    assert!(!session.stand_in(PlayerId::new(0)));
    assert!(!session.take_over(PlayerId::new(1)));
    assert!(session.answer_by_clock(PlayerId::new(0), 0).is_none());
    session.set_fair_scouting(false);
    assert!(session.fair_scouting());
    let public = serde_json::to_string(&session.game_static(PlayerId::new(0))).unwrap();
    assert!(!public.contains("challenge_policy_seed"));
    assert!(!public.contains("9001"));
    assert!(
        session.record_pending() > 0,
        "private record retains the configuration"
    );
}

#[test]
fn unsafe_challenge_execution_modes_are_refused_before_the_first_hand() {
    let preset = setup();
    let mut changed = preset.clone();
    changed.house_rules.reconnect_window_secs = 1;
    assert!(Session::new(&changed).is_none());
    changed = preset.clone();
    changed.house_rules.decision_timeout_secs = 1;
    assert!(Session::new(&changed).is_none());
    changed = preset.clone();
    changed.house_rules.takebacks = true;
    assert!(Session::new(&changed).is_none());
    changed = preset.clone();
    changed.seats[0].team = Some(1);
    assert!(Session::new(&changed).is_none());
    changed = preset.clone();
    changed.seats[1].controller = SeatController::Open;
    assert!(Session::new(&changed).is_none());
    changed = preset;
    changed.seats[1].controller = SeatController::Ai(AIProfile::EXPERT);
    assert!(
        Session::new(&changed).is_none(),
        "adaptive search is not a frozen policy"
    );
}
