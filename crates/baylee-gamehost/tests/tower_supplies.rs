//! Actual Session projections and Expert actions for original Tower supplies.
#![forbid(unsafe_code)]

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_cards_pool as _;
use baylee_core::ids::{PlayerId, PrintRef};
use baylee_core::preset::{DeckEntry, FormatId, GamePreset, SeatController, SeatSupplies};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::{CounterKind, ObjectStatus, Phase};

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn position(mana: bool) -> Session {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(18, entry("Forest").card).unwrap();
    preset.format = FormatId::Custom;
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.supplies = vec![SeatSupplies {
        seat: 0,
        healing: u8::from(!mana),
        mana: u8::from(mana),
        seedling: false,
    }];
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_life = Some(5);
    if mana {
        preset.seats[0].starting_hand = Some(vec![entry("Brainstorm")]);
    }
    let mut session = Session::new(&preset).unwrap();
    for _ in 0..40 {
        let pending = session.pending().clone();
        let player = pending.asked().unwrap();
        if matches!(pending, Pending::Priority { .. })
            && player == ME
            && session.observe_view(ME).phase == Phase::FirstMain
        {
            return session;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup: {other:?}"),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("first main not reached");
}

#[test]
fn tower_healing_emblem_projects_a_real_ability_and_expert_uses_the_paid_charge() {
    let mut session = position(false);
    let (pending, view) = session.view_for(ME).unwrap();
    let potion = &view.command[0][0];
    assert!(potion.token.is_some());
    assert!(!potion.status.contains(ObjectStatus::TOKEN));
    assert_eq!(potion.counter_count(CounterKind::Charge), 1);
    let action = HeuristicAgent::new(AIProfile::named("expert").unwrap()).act(&view, &pending);
    assert_eq!(
        action,
        PlayerAction::ActivateAbility {
            source: potion.id,
            ability_index: 0
        }
    );
    session.act_slice(ME, action, 0).unwrap();
    let view = session.observe_view(ME);
    assert_eq!(view.command[0][0].counter_count(CounterKind::Charge), 0);
    assert_eq!(view.stack.len(), 1);
}

#[test]
fn tower_expert_uses_real_mana_potion_color_question_to_cast_a_spell() {
    let mut session = position(true);
    let expert = HeuristicAgent::new(AIProfile::named("expert").unwrap());
    let mut chose_color = false;
    for _ in 0..8 {
        let (pending, view) = session.view_for(ME).unwrap();
        if !view.stack.is_empty() {
            assert!(chose_color);
            assert_eq!(view.command[0][0].counter_count(CounterKind::Charge), 0);
            assert!(view.stack.iter().any(|s| s.name == "Brainstorm"));
            return;
        }
        chose_color |= matches!(pending, Pending::ChooseColor { .. });
        session
            .act_slice(ME, expert.act(&view, &pending), 0)
            .unwrap();
    }
    panic!("potion did not enable the spell");
}
