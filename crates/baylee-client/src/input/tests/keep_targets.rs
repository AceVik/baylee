//! "Keep all remaining" on a copy's new-target questions (report 10): the press on the shelf's button, and the driver that answers for it.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::hud::PromptButton;
use baylee_client_core::ledge::PromptAction;
use baylee_core::ids::{DamageSourceRef, TargetRef};
use baylee_engine::choice::{PlayerAction, TargetPrompt};

/// Seat 0 asked about target `index` of `of` of a copy, `min` as the engine
/// sets it.
fn asked(min: u32, index: u32, of: u32) -> crate::Duel {
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let me = view.seat;
    let pending = Pending::ChooseTargets {
        player: me,
        options: vec![obj(9)],
        player_options: vec![],
        min,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: TargetRef::Object(DamageSourceRef {
                object: obj(9),
                version: 1,
            }),
            index,
            of,
        },
    };
    crate::Duel {
        view: Some(view),
        interaction: Some(Interaction::new(pending, me)),
        ..crate::Duel::default()
    }
}

/// The pointer, then the autopilot that submits a standing instruction's
/// answer, with a "keep all remaining" button to press.
fn app_with_the_button(duel: crate::Duel) -> (bevy::app::App, bevy::prelude::Entity) {
    use bevy::prelude::*;
    let mut app = pointer_app(duel);
    app.add_systems(Update, crate::run_autopilot.after(crate::input::pointer));
    let button = app
        .world_mut()
        .spawn(PromptButton {
            decision_id: None,
            action: PromptAction::KeepTargets,
        })
        .id();
    (app, button)
}

fn keep() -> PlayerAction {
    PlayerAction::ChooseTargets {
        objects: Vec::new(),
        players: Vec::new(),
    }
}

/// The press only approves the run; the driver then sends the empty answer
/// the engine's question offers, once, and the approval stays for the
/// questions after it.
#[test]
fn pressing_keep_all_answers_this_question_empty_and_holds_the_run() {
    let (mut app, button) = app_with_the_button(asked(0, 0, 3));
    click(&mut app, button);
    let duel = app.world().resource::<crate::Duel>();
    assert_eq!(
        duel.outbox(),
        [keep()],
        "the press sent no empty ChooseTargets answer"
    );
    assert!(
        duel.keep_targets.is_some(),
        "the run was dropped after its first answer"
    );
    app.world_mut().resource_mut::<crate::Duel>().take_outbox();
    app.update();
    assert!(
        app.world().resource::<crate::Duel>().outbox().is_empty(),
        "the same question was answered a second time"
    );
}

/// The next question of the run is answered with no press at all, and a
/// question that must be changed is left for the player.
#[test]
fn the_run_answers_the_next_question_and_leaves_a_must_change_one() {
    let (mut app, button) = app_with_the_button(asked(0, 0, 3));
    click(&mut app, button);
    let next = |app: &mut bevy::app::App, duel: crate::Duel| {
        let mut world_duel = app.world_mut().resource_mut::<crate::Duel>();
        let view = duel.view.clone().expect("a view");
        let pending = duel
            .interaction
            .as_ref()
            .expect("a question")
            .pending()
            .clone();
        world_duel.take_outbox();
        world_duel.receive_view(view);
        world_duel.receive_choice(pending);
        app.update();
        app.world_mut().resource_mut::<crate::Duel>().take_outbox()
    };
    let mut second = asked(0, 1, 3);
    second.view.as_mut().expect("a view").seq = 2;
    assert_eq!(next(&mut app, second), [keep()], "target 2 of 3 kept");
    let mut third = asked(1, 2, 3);
    third.view.as_mut().expect("a view").seq = 3;
    assert!(
        next(&mut app, third).is_empty(),
        "a question that must be changed was answered for the player"
    );
}

/// On the last target there is nothing to keep ahead; a press (a stale
/// button) approves nothing and sends nothing.
#[test]
fn pressing_keep_all_on_the_last_target_approves_nothing() {
    let (mut app, button) = app_with_the_button(asked(0, 2, 3));
    click(&mut app, button);
    let duel = app.world().resource::<crate::Duel>();
    assert!(
        duel.keep_targets.is_none(),
        "a run began on the last target"
    );
    assert!(duel.outbox().is_empty(), "sent: {:?}", duel.outbox());
}
