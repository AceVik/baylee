//! The prompt bar: its answer buttons, the drawer, holds and waits.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// Every word the bar put on the screen.
///
/// `TextSpan` and not `Text`: `slip_text` splits a line into runs so a
/// bracketed aside can be greyed, which leaves the `Text` itself empty
/// and every word in a child.
#[test]
fn combat_commit_buttons_update_without_a_new_game_snapshot() {
    use baylee_core::ids::Defender;
    use baylee_engine::choice::Pending;
    for blocking in [false, true] {
        let creature = ObjectId::new(3, 0);
        let attacker = ObjectId::new(4, 0);
        let pending = if blocking {
            Pending::ChooseBlockers {
                demands: Vec::new(),
                player: PlayerId::new(0),
                attacker: PlayerId::new(1),
                blockers: vec![baylee_engine::choice::BlockOption {
                    blocker: creature,
                    attackers: vec![attacker],
                }],
                capacity: Vec::new(),
                obeying: Vec::new(),
                bounds: Vec::new(),
            }
        } else {
            Pending::ChooseAttackers {
                player: PlayerId::new(0),
                attackers: vec![creature],
                defenders: vec![Defender::Player(PlayerId::new(1))],
                required: Vec::new(),
                limits: Vec::new(),
            }
        };
        let mut duel = duel_with(false);
        duel.receive_choice(pending);
        let mut app = bar_of(duel);
        let label = if blocking {
            Phrase::Block
        } else {
            Phrase::Attack
        }
        .text(Lang::En);
        assert!(!said(&mut app).iter().any(|s| s == label));
        {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            let i = duel.interaction.as_mut().unwrap();
            if blocking {
                assert!(i.declare_blocker(creature, attacker));
            } else {
                assert!(i.declare_attacker(creature, Defender::Player(PlayerId::new(1))));
            }
        }
        app.update();
        assert!(said(&mut app).iter().any(|s| s == label), "missing {label}");
    }
}

#[test]
fn tray_image_arrivals_preserve_entities_and_scroll_and_relayout_keeps_offset() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.open();
    let mut app = bar_of(duel);
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<tray::TrayScroll>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(list)
        .get_mut::<ScrollPosition>()
        .unwrap()
        .y = 160.0;
    app.world_mut().resource_mut::<CardTextures>().mark_arrived(
        baylee_client_core::images::ImageKey::new(
            baylee_core::ids::PrintRef::new(55),
            0,
            baylee_client_core::images::ArtSize::Small,
        ),
    );
    app.update();
    assert!((app.world().get::<ScrollPosition>(list).unwrap().y - 160.0).abs() < f32::EPSILON);
    app.world_mut()
        .resource_mut::<tray::TrayRevision>()
        .relayout();
    app.update();
    let scroll = app
        .world_mut()
        .query_filtered::<&ScrollPosition, With<tray::TrayScroll>>()
        .single(app.world())
        .unwrap();
    assert!((scroll.y - 160.0).abs() < f32::EPSILON);
}

pub(super) fn said(app: &mut App) -> Vec<String> {
    let mut roots = app.world_mut().query::<&Text>();
    let mut lines: Vec<String> = roots.iter(app.world()).map(|t| t.0.clone()).collect();
    let mut spans = app.world_mut().query::<&TextSpan>();
    lines.extend(spans.iter(app.world()).map(|s| s.0.clone()));
    lines
}

/// #182/#140: selecting only a player must rebuild the confirmation row.
#[test]
fn selecting_a_player_refreshes_the_confirm_button() {
    use baylee_engine::choice::{Pending, TargetPrompt};
    let mut duel = duel_saying(false, false);
    duel.last_error = None;
    duel.interaction = Some(baylee_client_core::Interaction::new(
        Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![],
            player_options: vec![PlayerId::new(0), PlayerId::new(1)],
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        PlayerId::new(0),
    ));
    let mut app = bar_of(duel);
    let confirms = |app: &mut App| {
        app.world_mut()
            .query::<&PromptButton>()
            .iter(app.world())
            .filter(|button| button.action == PromptAction::Confirm)
            .count()
    };
    assert_eq!(confirms(&mut app), 0);
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .toggle_player(PlayerId::new(1));
    app.update();
    assert_eq!(confirms(&mut app), 1);
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .toggle_player(PlayerId::new(1));
    app.update();
    assert_eq!(confirms(&mut app), 0);
}

/// The drawer opens on a question whose answer is a list, and is shut the
/// rest of the time.
///
/// Both halves, because each is a different bug. A drawer that never
/// opens is a colour chooser drawn as "Choose a colour" with nothing
/// under it, which is how a tapped dual land used to stop a game dead. A
/// drawer that never shuts is an empty panel standing over the table for
/// the whole of every turn — and its node is exempt from the overlay's
/// sweep now, so nothing else would take it away.
///
/// It counts [`ChoiceButton`]s rather than reading words: a colour is
/// answered by a mana pip and carries no label at all, on the grounds
/// that a `{U}` disc says "blue" in every language there is.
#[test]
fn the_drawer_opens_on_a_list_and_is_shut_otherwise() {
    let rows = |app: &mut App| {
        let mut q = app.world_mut().query::<&ChoiceButton>();
        q.iter(app.world()).map(|b| b.index).collect::<Vec<_>>()
    };
    let panels = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<Option<&Children>, With<ledge::drawer::DrawerRoot>>();
        q.iter(app.world())
            .map(|c| c.map_or(0, bevy::ecs::hierarchy::Children::len))
            .sum::<usize>()
    };

    let mut app = bar_of(duel_choosing_a_colour());
    assert_eq!(
        rows(&mut app),
        vec![0, 1, 2],
        "three colours were offered and the drawer has to carry all three"
    );
    assert_eq!(panels(&mut app), 1, "and they stand on one panel");

    // The same seat, with nothing being asked of it.
    let mut app = bar_of(duel_with(false));
    assert!(
        rows(&mut app).is_empty(),
        "a priority window is answered on the shelf, not out of a drawer"
    );
    assert_eq!(
        panels(&mut app),
        0,
        "an empty drawer is a panel standing over the table saying nothing"
    );
}

/// The panel is kept while the question is, and it leaves with it.
///
/// §7 gives the drawer the one movement on this shelf that overshoots:
/// it *appears*, where an answer merely *changes*. That only reads right
/// if "appears" happens once — a panel rebuilt whenever its contents
/// changed would pop again on the row a player has just taken, which is a
/// movement saying "something new arrived" about the click they just
/// made. So the identity of the entity is the assertion: the same panel
/// across a change of contents, and **no** panel once the question is
/// gone. The second half is what
/// [`the_drawer_opens_on_a_list_and_is_shut_otherwise`] cannot reach —
/// it builds a fresh app per case, so nothing there has ever had to
/// leave.
#[test]
fn the_drawers_panel_outlives_its_contents_and_not_its_question() {
    let panel = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<&Children, With<ledge::drawer::DrawerRoot>>();
        q.iter(app.world()).flatten().copied().next()
    };

    let mut app = bar_of(duel_choosing_a_colour());
    let opened = panel(&mut app).expect("a colour is chosen out of the drawer");

    // The middle colour is taken. The rows are redrawn — one of them is
    // washed now — and the drawer has not opened again.
    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        assert!(
            duel.interaction.as_mut().is_some_and(|i| i.choose_index(1)),
            "the second colour is one of the three that were offered"
        );
    }
    app.update();
    assert_eq!(
        panel(&mut app),
        Some(opened),
        "a row being taken is a change of contents, not a second arrival"
    );

    // And the question goes away. With motion off the way out is over on
    // the frame it starts, so the panel is gone by the end of this one.
    *app.world_mut().resource_mut::<Duel>() = duel_with(false);
    app.update();
    assert_eq!(
        panel(&mut app),
        None,
        "the drawer has to be able to leave, not merely to stop being filled"
    );
}

/// A running hold says so in the middle, where the question would be.
///
/// It is the one game state with **no other symptom** (AX §4.4): the
/// middle is empty precisely *because* the seat is not being asked, which
/// is what an idle shelf looks like — so a player who set a hold two
/// turns ago and forgot would watch the game play itself with nothing on
/// screen to blame.
///
/// Two mechanisms and one picture, and this is where that is asserted
/// rather than merely intended: an engine hold and the client's own
/// autopilot each put the same sentence and the same way out on the
/// shelf. Live, neither is a state a screenshot can be relied on to
/// catch — the house AI answers a whole turn between two frames.
#[test]
fn a_running_hold_is_drawn_where_the_question_would_have_been() {
    for (hold, pilot, what) in [
        (true, false, "an engine hold"),
        (false, true, "the autopilot"),
    ] {
        let mut app = bar_of(duel_not_asking(hold, pilot));
        let lines = said(&mut app);
        for phrase in [Phrase::HoldingPriority, Phrase::HoldRelease] {
            let words = phrase.text(Lang::En).to_string();
            assert!(
                lines.contains(&words),
                "{what} left the shelf with nothing to blame: {lines:?}"
            );
        }
    }
    // The counter-half, and the reason the sentence is not simply always
    // there: a seat that *is* being asked has a question of its own, and
    // two sentences on one shelf is what §6 forbids.
    let mut app = bar_of(duel_not_asking(false, false));
    let lines = said(&mut app);
    let words = Phrase::HoldingPriority.text(Lang::En).to_string();
    assert!(
        !lines.contains(&words),
        "nobody is holding anything and the shelf said otherwise: {lines:?}"
    );
}

/// A seat that has kept its opening hand while another is still deciding
/// (#257) holds no question — a host sends each seat only its own — and
/// the shelf says who it is waiting on instead of standing empty.
///
/// Run through the ledge and read off its tree, because the sentence is
/// a third source behind the cast chooser and the question
/// (`Duel::headline`) and the join is the part that can be left out. The
/// counter-half is turn 1: the same seat, holding no question, with
/// `deciding` empty, says nothing of the kind.
#[test]
fn a_seat_that_has_kept_is_told_who_is_still_deciding() {
    let shelf = |deciding: &[u8]| {
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.awaiting = None;
        view.deciding = deciding.iter().copied().map(PlayerId::new).collect();
        let mut statics = baylee_client_core::test_support::statics(8);
        statics.seats.push(baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "sharp 1".to_string(),
            is_ai: false,
            away: false,
            team: None,
        });
        let mut duel = Duel {
            view: Some(view),
            statics: Some(statics),
            ..Duel::default()
        };
        crate::rebuild_board(&mut duel);
        said(&mut bar_of(duel))
    };
    let kept = shelf(&[1]);
    assert!(
        kept.contains(&"Waiting for sharp 1".to_string()),
        "the shelf names the seat still deciding: {kept:?}"
    );
    let turn_one = shelf(&[]);
    assert!(
        !turn_one.iter().any(|line| line.starts_with("Waiting for")),
        "nobody is deciding and the shelf said somebody was: {turn_one:?}"
    );
}
