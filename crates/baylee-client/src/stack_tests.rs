//! Stack selection is navigation, not a target choice or a priority answer.
use super::*;
use baylee_client_core::automation::{AbilityOrder, set_ability_order};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{AbilityRef, CardIndex};
use baylee_engine::choice::{LegalActions, PriorityHold, StandingAnswer, TargetPrompt};

fn duel_with_stack() -> Duel {
    let mut view = ViewBuilder::new(2).build();
    view.stack = (100..120)
        .map(|id| {
            let mut object = token(id, 0, "Stack source", 1, 1);
            object.stack_item = Some(baylee_view::StackItem::Spell);
            object
        })
        .collect();
    view.awaiting = Some(view.seat);
    let mut duel = Duel::default();
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            ..default()
        }),
    });
    duel
}

#[test]
fn selecting_a_stack_row_marks_a_stop_without_sending_an_action() {
    let mut duel = duel_with_stack();
    let mark = duel.view.as_ref().unwrap().stack[3].id;
    crate::input::activate_card(&mut duel, mark);
    assert_eq!(duel.stack_selected, Some(mark));
    assert!(duel.outbox().is_empty());
    assert_eq!(
        duel.hold_action(false),
        Some(PlayerAction::SetPriorityHold(
            PriorityHold::UntilTopOfStack { object: mark }
        ))
    );
    crate::input::activate_card(&mut duel, mark);
    assert_eq!(duel.stack_selected, None);
    assert_eq!(
        duel.hold_action(false),
        Some(PlayerAction::SetPriorityHold(
            PriorityHold::UntilStackEmpty { depth: 20 }
        ))
    );
}

#[test]
fn target_selection_takes_precedence_over_stack_navigation() {
    let mut duel = duel_with_stack();
    let target = duel.view.as_ref().unwrap().stack[5].id;
    duel.receive_choice(Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![target],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    crate::input::activate_card(&mut duel, target);
    assert!(duel.interaction.as_ref().unwrap().is_selected(target));
    assert_eq!(duel.stack_selected, None);
}

#[test]
fn removed_marks_cannot_redirect_a_later_stack_hold() {
    let mut duel = duel_with_stack();
    duel.stack_selected = Some(duel.view.as_ref().unwrap().stack[0].id);
    let mut view = duel.view.clone().unwrap();
    view.stack.remove(0);
    duel.receive_view(view);
    assert_eq!(duel.stack_selected, None);
    assert_eq!(
        duel.hold_action(false),
        Some(PlayerAction::SetPriorityHold(
            PriorityHold::UntilStackEmpty { depth: 19 }
        ))
    );
}

#[test]
fn reaching_a_mark_does_not_fall_through_to_client_autopassing() {
    let mut duel = duel_with_stack();
    duel.stack_selected = Some(duel.view.as_ref().unwrap().stack[0].id);
    duel.submit(duel.hold_action(false).unwrap());
    duel.take_outbox();
    let mut app = App::new();
    app.insert_resource(duel)
        .init_resource::<prefs::Prefs>()
        .add_systems(Update, run_autopilot);
    app.update();
    assert!(app.world().resource::<Duel>().outbox().is_empty());
    let mut duel = app.world_mut().resource_mut::<Duel>();
    duel.submit(PlayerAction::PassPriority);
    assert!(!duel.stack_stop_requested);
}

#[test]
fn remembered_rules_are_installed_once_and_cleared_in_the_engine() {
    let ability = AbilityRef::new(CardIndex::new(12), 2);
    let order = AbilityOrder {
        ability,
        pass: true,
        answer: Some(StandingAnswer::Yes),
    };
    let mut prefs = prefs::Prefs::default();
    set_ability_order(&mut prefs.edit().ability_orders, order);
    let mut app = App::new();
    let mut duel = duel_with_stack();
    duel.interaction = None;
    app.insert_resource(duel)
        .insert_resource(prefs)
        .add_systems(Update, run_autopilot);
    app.update();
    assert_eq!(
        app.world_mut().resource_mut::<Duel>().take_outbox(),
        [order.action()]
    );
    app.update();
    assert!(app.world().resource::<Duel>().outbox().is_empty());
    app.world_mut()
        .resource_mut::<prefs::Prefs>()
        .edit()
        .ability_orders
        .clear();
    app.update();
    assert_eq!(
        app.world_mut().resource_mut::<Duel>().take_outbox(),
        [AbilityOrder::manual(ability).action()]
    );
}

#[test]
fn an_intervening_optional_answer_keeps_the_requested_stack_boundary() {
    let mut duel = duel_with_stack();
    duel.stack_selected = Some(duel.view.as_ref().unwrap().stack[0].id);
    duel.submit(duel.hold_action(false).unwrap());
    duel.take_outbox();
    duel.interaction = None;
    duel.submit(PlayerAction::YesNo(true));
    assert!(duel.stack_stop_requested);
    duel.submit(PlayerAction::SetPriorityHold(PriorityHold::Always));
    assert!(
        duel.stack_stop_requested,
        "cancel also requires a manual priority response"
    );
}
