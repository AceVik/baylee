//! The sheets: casting, putting away, dismissing, maximising.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// Whether `entity` hangs somewhere under a standing parchment leaf.
///
/// Ancestry and not a count, because AX 6c moved rows between two panels
/// that draw the **same component**: a cast row is one row of an indexed
/// choice on the sheet exactly as a colour is one in the drawer, and both
/// wear a [`ChoiceButton`]. Counting them would pass whichever panel had
/// them, which is the one thing this is about.
fn under_a_sheet(app: &App, entity: Entity) -> bool {
    let mut at = entity;
    loop {
        let found = app.world().entity(at);
        if found.contains::<AbilitySheetRoot>() {
            return true;
        }
        let Some(parent) = found.get::<ChildOf>().map(ChildOf::parent) else {
            return false;
        };
        at = parent;
    }
}

/// Every indexed-choice row on the screen, and where it is standing.
fn choice_rows(app: &mut App) -> Vec<(usize, bool)> {
    let mut q = app.world_mut().query::<(Entity, &ChoiceButton)>();
    let found: Vec<(Entity, usize)> = q
        .iter(app.world())
        .map(|(entity, button)| (entity, button.index))
        .collect();
    found
        .into_iter()
        .map(|(entity, index)| (index, under_a_sheet(app, entity)))
        .collect()
}

/// How many panels the drawer is holding.
fn drawer_panels(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Option<&Children>, With<ledge::drawer::DrawerRoot>>();
    q.iter(app.world())
        .map(|c| c.map_or(0, bevy::ecs::hierarchy::Children::len))
        .sum()
}

/// A seat holding one card it can cast two ways, with **this client's
/// own** chooser standing open over it.
///
/// `CastMenu` is a `Prompt::CastMode` built one step before the engine
/// would have built one, while the engine is still holding an ordinary
/// priority window. Two modes and no card text, because
/// `choices::cast_label` answers `Normal` and `Alternative` out of
/// `Phrase` when the printing has not arrived — which is the state this
/// harness is always in.
fn duel_casting() -> Duel {
    use baylee_engine::choice::CastModeKind;
    let mut duel = duel_with(false);
    duel.view = Some(
        baylee_client_core::test_support::ViewBuilder::new(2)
            .with_hand(vec![("Fire", 2, 4)])
            .build(),
    );
    crate::rebuild_board(&mut duel);
    duel.cast_menu = Some(crate::CastMenu {
        card: ObjectId::new(4, 0),
        modes: vec![
            crate::castmodes::ReachableMode {
                kind: CastModeKind::Normal,
                cost: ManaCost::default(),
                plan: baylee_client_core::manaplan::Plan::default(),
            },
            crate::castmodes::ReachableMode {
                kind: CastModeKind::Alternative(0),
                cost: ManaCost::default(),
                plan: baylee_client_core::manaplan::Plan::default(),
            },
        ],
        pick: 0,
    });
    duel
}

/// The ways of casting a card stand on the card's own sheet, and the
/// drawer keeps the questions with no card to stand beside.
///
/// The owner's answer of 14.09.2026, and AX step 6c: the cast-mode
/// chooser is the **same piece of parchment** as the ability chooser. §5
/// had put the indexed chooser in the drawer and left the ability one
/// beside the card, and the sentence §5 gave for that — *it belongs to
/// the card, not to the question* — is just as true of this one.
///
/// The colour half is what stops the sheet swallowing every indexed
/// choice there is. A colour has no card: the question comes off a mana
/// ability that is already resolving, so there is nothing on the table to
/// hang paper beside, and a rule that moved *all* `ChoiceButton`s onto a
/// sheet would have nowhere to put it.
#[test]
fn the_ways_to_cast_a_card_stand_on_its_sheet_and_not_in_the_drawer() {
    let mut app = bar_of(duel_casting());
    let rows = choice_rows(&mut app);
    assert_eq!(
        rows,
        vec![(0, true), (1, true)],
        "both ways of casting the card have to be on the leaf beside it"
    );
    assert_eq!(
        drawer_panels(&mut app),
        0,
        "the rows moved to the sheet, so the drawer is an empty panel \
         standing over the table saying nothing"
    );

    // And the question with no card to stand beside stayed where it was.
    let mut app = bar_of(duel_choosing_a_colour());
    let rows = choice_rows(&mut app);
    assert_eq!(
        rows,
        vec![(0, false), (1, false), (2, false)],
        "a colour is answered out of the drawer — it comes off an ability \
         that is already resolving and has no card to hang paper beside"
    );
    assert_eq!(drawer_panels(&mut app), 1, "and they stand on one panel");
}

/// The same question drawn in the same place however it arrived.
///
/// This client asks first, but the engine asks `ChooseCastMode` itself
/// whenever the client did not get there first — a modal trigger, a
/// pathway, a seat driven over the wire. Both are `Prompt::CastMode` and
/// both are about a card, so a chooser that stood beside the card on one
/// route and in the drawer on the other would be one question moving
/// depending on how it had arrived.
///
/// It is also the half that fails against the code this replaced: the
/// drawer stopped reading `Prompt::CastMode` at all, so a sheet that read
/// only `Duel::cast_menu` would draw this question **nowhere**.
///
/// The cross is the second assertion, and it is the difference the two
/// routes really do have. Every door out of this sheet works by clearing
/// the menu that opened it; a question the engine asked is one the table
/// is waiting on, so there is nothing to clear and a cross there would be
/// a control that visibly does nothing.
#[test]
fn a_cast_question_the_engine_asked_lands_on_the_same_sheet() {
    use baylee_engine::choice::{CastModeDesc, CastModeKind};

    let crosses = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<SheetClose>>();
        q.iter(app.world()).count()
    };

    let mut duel = duel_casting();
    duel.cast_menu = None;
    duel.interaction = Some(baylee_client_core::Interaction::new(
        baylee_engine::choice::Pending::ChooseCastMode {
            player: PlayerId::new(0),
            object: ObjectId::new(4, 0),
            options: vec![
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: ManaCost::default(),
                },
                CastModeDesc {
                    index: 1,
                    kind: CastModeKind::Alternative(0),
                    cost: ManaCost::default(),
                },
            ],
        },
        PlayerId::new(0),
    ));
    let mut app = bar_of(duel);
    assert_eq!(
        choice_rows(&mut app),
        vec![(0, true), (1, true)],
        "the engine asked the question this time, and it is the same \
         question about the same card"
    );
    assert_eq!(
        drawer_panels(&mut app),
        0,
        "and it is not drawn twice, nor left in the drawer it came from"
    );
    assert_eq!(
        crosses(&mut app),
        0,
        "a question the table is waiting on cannot be put down, so the \
         cross on it would be a door to nowhere"
    );

    // The counter-half: the cross is drawn where there *is* something to
    // clear, or the assertion above would pass on a sheet that never had
    // one.
    let mut app = bar_of(duel_casting());
    assert_eq!(
        crosses(&mut app),
        1,
        "this client's own chooser can be put down, and the cross is how"
    );
}

/// A sheet put away stands for as long as its flight into the tray, and
/// then it is gone.
///
/// Both halves, and the second is what makes the first mean anything: a
/// dialog that simply stopped being despawned would pass "still standing"
/// for ever, and the whole reason `sync_tray` hands the sheet to
/// `reveal_tray` rather than tearing it down is that something else takes
/// it off the tree at the end.
///
/// Motion is turned back **on** for this test. The harness runs with
/// `reduce_motion`, where the flight is over on the frame it starts —
/// which is what keeps every test written before it meaning what it
/// meant, and which would make this one unable to see the movement at
/// all.
#[test]
fn a_sheet_put_away_flies_to_the_tray_before_it_stops_existing() {
    use std::time::Duration;

    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.open();
    let mut app = bar_of(duel);
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;

    let bands = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
        q.iter(app.world()).count()
    };
    assert_eq!(bands(&mut app), 1, "the sheet was opened and is drawn");

    // The player presses the tray button. The browser is shut on this
    // very frame; the sheet is not.
    app.world_mut().resource_mut::<Duel>().browser.close();
    app.update();
    assert_eq!(
        bands(&mut app),
        1,
        "the sheet was despawned on the frame the browser shut, so there \
         is nothing left for the flight to move"
    );

    // A frame that is not long enough, so that "it went" is about the
    // span and not about the next `update` whenever it happens.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(40));
    app.update();
    assert_eq!(bands(&mut app), 1, "it left before its flight was over");

    // And past the end of it.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(400));
    app.update();
    assert_eq!(
        bands(&mut app),
        0,
        "the sheet is still on the tree with nothing left to move it"
    );
}

/// A sheet a **question** opened does not fly: it is dismissed where it
/// stands.
///
/// The other arm of `reveal_tray`'s match, and the one with nowhere to
/// go — the tray's button is *held* for as long as the question owns the
/// sheet, so a flight to it would be a movement towards a door the player
/// was not allowed through. `hud::motion`'s own rule is that a dismissal
/// is not a journey, and this takes `motion::shutting`, the curve the
/// drawer is dismissed on.
///
/// Which arm is taken is decided from `TrayReveal::to_tray`, recorded
/// while the sheet is **up**: by the time it closes the browser is shut,
/// and a shut browser answers `for_choice` with false. So the `follow(…,
/// None)` below is doing two jobs — it answers the question, and it is
/// the only route by which this arm is reachable at all.
///
/// Motion is turned back on for its sibling's reason: under
/// `reduce_motion` the whole movement happens in the frame it starts and
/// there is nothing left to read.
#[test]
fn an_answered_sheet_is_dismissed_where_it_stands() {
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_engine::choice::{ChoicePrompt, Pending};
    use std::time::Duration;

    let view = ViewBuilder::new(2).build();
    let asked = baylee_client_core::interaction::Interaction::new(
        Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(7, 0)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
            total: None,
        },
        PlayerId::new(0),
    );

    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.follow(&view, Some(&asked));
    assert!(
        duel.browser.for_choice(),
        "the harness did not open the sheet for a question"
    );

    let mut app = bar_of(duel);
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;

    // Answered: the next question wants nothing from the sheet, which is
    // `Browser::follow`'s one exception to leaving an open sheet alone.
    app.world_mut()
        .resource_mut::<Duel>()
        .browser
        .follow(&view, None);
    app.update();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(40));
    app.update();

    let read: Vec<(Val2, Vec2)> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&UiTransform, With<TrayPanel>>();
        q.iter(app.world())
            .map(|t| (t.translation, t.scale))
            .collect()
    };
    assert_eq!(
        read.len(),
        1,
        "the answered sheet is not on the tree to be read, so nothing \
         below would be a measurement of anything"
    );
    let (translation, scale) = read[0];
    assert_eq!(
        translation,
        Val2::new(px(0.0), px(0.0)),
        "an answered sheet travelled, and it has nowhere to travel to"
    );
    assert!(
        scale.x < 1.0,
        "an answered sheet did not shrink at all: {scale:?}"
    );
}

/// A sheet caught on its way out is not turned round: it goes, and a
/// fresh one opens in its place. Either way there is exactly **one**.
///
/// The count is the assertion. A flight runs on its own span and against
/// its own curve, so `sync_tray` despawns whatever is leaving rather than
/// reversing it — and the failure that shape is guarding against is two
/// sheets on the tree at once, one of them shrinking towards the tray
/// while the other one draws the rows.
#[test]
fn a_sheet_reopened_mid_flight_is_one_sheet_and_not_two() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.open();
    let mut app = bar_of(duel);
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;

    let bands = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    let was = bands(&mut app);
    assert_eq!(was.len(), 1, "one sheet to begin with");

    app.world_mut().resource_mut::<Duel>().browser.close();
    app.update();
    app.world_mut().resource_mut::<Duel>().browser.open();
    app.update();

    let now = bands(&mut app);
    assert_eq!(now.len(), 1, "a sheet was caught and turned round: {now:?}");
    assert!(
        now[0] != was[0],
        "the leaving sheet was kept and refilled, so it is still running \
         the closing curve with the rows of a question that came back"
    );
}

/// A maximised sheet offers to **restore**, and a normal one to maximise.
///
/// The mark is chosen at build time from `place.is_maximised(band)`, and
/// the placement is deliberately not one of `TrayRevision`'s fields — a
/// drag writes it every frame and rebuilding the sheet per pixel would
/// make it unusable. So resizing the sheet leaves the head drawn from a
/// rectangle that is no longer true, and a maximised sheet went on
/// offering to maximise until something unrelated caused a rebuild.
/// `TrayRevision::relayout` is the one word that says "the size stopped
/// changing", and this is the assertion it exists for: without the
/// `!stale` term in the gate the second half of this test reads the first
/// half's tree.
///
/// Two marks and not one toggled ink, because a control offering what it
/// cannot do is the lie this dialog already refuses to tell with a lit
/// tab or an unlit Confirm.
#[test]
fn a_compact_target_sheet_draws_only_the_offer_and_explicit_confirmation() {
    use baylee_engine::choice::{Pending, TargetPrompt};
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_stack(vec![baylee_client_core::test_support::printed(
            1, 0, "Spell", 1,
        )])
        .build();
    let mut duel = Duel {
        view: Some(view),
        statics: Some(baylee_client_core::test_support::statics(8)),
        ..Duel::default()
    };
    duel.receive_choice(Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![ObjectId::new(1, 0)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    let mut app = bar_of(duel);
    assert_eq!(
        app.world_mut()
            .query::<&TrayCard>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&TrayFilter>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&TrayTab>()
            .iter(app.world())
            .count(),
        0
    );
    let panel = app
        .world_mut()
        .query_filtered::<&Node, With<TrayPanel>>()
        .single(app.world())
        .unwrap();
    assert!(matches!(panel.width, Val::Px(w) if w <= 640.0));
    assert!(matches!(panel.height, Val::Px(h) if h < 300.0));
    assert!(app.world().resource::<Duel>().outbox.is_empty());
    let confirm_count = |app: &mut App| {
        app.world_mut()
            .query::<&PromptButton>()
            .iter(app.world())
            .filter(|b| b.action == PromptAction::Confirm)
            .count()
    };
    assert_eq!(confirm_count(&mut app), 0);
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .toggle(ObjectId::new(1, 0));
    app.update();
    assert_eq!(confirm_count(&mut app), 1);
}

#[test]
fn a_maximised_sheet_offers_to_put_itself_back() {
    use baylee_client_core::browser::Placement;

    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.open();
    let mut app = bar_of(duel);

    let marks = |app: &mut App| {
        let said = said(app);
        (
            said.iter().any(|t| t.contains(glyph::MAXIMISE)),
            said.iter().any(|t| t.contains(glyph::RESTORE)),
        )
    };
    assert_eq!(
        marks(&mut app),
        (true, false),
        "a sheet at its opening size offers the wrong thing"
    );

    // The sheet is maximised the way the button maximises it: the store
    // gets the band, and the movement that wrote it says it has stopped.
    // `band_of` has no window here and answers with its own fallback,
    // which is the band this app is laid out in.
    let band = (1280.0, 720.0 - EDGE - hand::HAND_ZONE_H);
    app.world_mut()
        .resource_mut::<crate::settings::ClientSettings>()
        .zone_browser = Some(Placement::maximised(band));
    app.world_mut()
        .resource_mut::<tray::TrayRevision>()
        .relayout();
    app.update();
    assert_eq!(
        marks(&mut app),
        (false, true),
        "the sheet fills the band and its head still offers to fill it"
    );
}

/// The end screen's veil is not the zone dialog's, and a rebuild of the
/// one does not take the other.
///
/// `hud::finish` spawns a `TableVeil` of its own under its own root, and
/// `sync_tray` used to tear down every `TableVeil` there was — so a game
/// that ended while anything about the dialog changed lost its
/// darkening, and `dim_the_table` then had no node to paint. It was never
/// observed, which is the reason to pin it: the two surfaces are painted
/// by one system on purpose and owned by two, and only a marker can say
/// which is which.
#[test]
fn the_end_screens_veil_is_not_torn_down_with_the_dialogs() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    let mut app = bar_of(duel);

    // Exactly as `hud::finish` does it: the shared constructor, and no
    // `TrayVeil` on top of it.
    let theirs = {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let id = {
            let mut commands = Commands::new(&mut queue, app.world());
            tray::spawn_veil(&mut commands)
        };
        queue.apply(app.world_mut());
        id
    };

    // Something about the dialog changes, which is what makes `sync_tray`
    // run its teardown at all.
    app.world_mut().resource_mut::<Duel>().browser.open();
    app.update();
    assert!(
        app.world().get_entity(theirs).is_ok(),
        "opening the zone dialog despawned the end screen's veil"
    );

    // And the counter-test: the dialog's own veil *is* torn down, so the
    // assertion above is about the marker and not about a teardown that
    // has quietly stopped happening.
    let mut ours = app.world_mut().query_filtered::<Entity, With<TrayVeil>>();
    let ours: Vec<Entity> = ours.iter(app.world()).collect();
    assert_eq!(ours.len(), 1, "the dialog drew a veil of its own");
    app.world_mut()
        .resource_mut::<Duel>()
        .browser
        .push_filter('a');
    app.update();
    let mut now = app.world_mut().query_filtered::<Entity, With<TrayVeil>>();
    let now: Vec<Entity> = now.iter(app.world()).collect();
    assert!(
        now.len() == 1 && now[0] != ours[0],
        "the dialog's own veil survived its rebuild, so this test would \
         pass on a teardown that despawns nothing at all"
    );
}

/// The stack has an answer of its own, and it is only offered while there
/// is a stack.
///
/// Three mechanisms stand in that row and look alike deliberately — the
/// engine's `Pass`, an engine hold, and a client-side autopilot — so the
/// one thing a test can check is that the middle of them appears exactly
/// when it does something. On an empty stack `hold_action(false)` sends
/// `UntilStackEmpty { depth: 0 }`, a hold that ends on the frame it
/// begins.
///
/// The cap it wears is `ledge::a_command_wears_the_key_that_does_the_same
/// _thing`'s and not this test's: a headless app has no `Window`, so the
/// shelf is arranged against the 1200-pixel fallback and spends that rung
/// on the keycaps first — the caps are legitimately absent here.
#[test]
fn the_stack_can_be_let_go_of_only_while_there_is_one() {
    let held = Phrase::ResolveTheStack.text(Lang::En).to_string();

    let mut empty = bar_of(duel_saying(false, false));
    let lines = said(&mut empty);
    assert!(
        !lines.contains(&held),
        "a hold until an empty stack is empty promises nothing: {lines:?}"
    );

    let mut app = bar_of(duel_with_a_stack());
    let lines = said(&mut app);
    assert!(
        lines.contains(&held),
        "there is a stack, so there is something to let resolve: {lines:?}"
    );

    // And it is a `MenuButton`, not an answer: the engine was not asked
    // this. `menu_click` is the other half — it re-reads the same
    // predicate before sending.
    let mut q = app
        .world_mut()
        .query_filtered::<&MenuButton, With<crate::ambience::Feel>>();
    let kinds: Vec<MenuAction> = q.iter(app.world()).map(|b| b.action).collect();
    assert!(
        kinds.contains(&MenuAction::HoldForStack),
        "the button carries the deed it does: {kinds:?}"
    );
    let mut prompts = app.world_mut().query::<&PromptButton>();
    let answers: Vec<PromptAction> = prompts
        .iter(app.world())
        .map(|b| b.action)
        .collect::<Vec<_>>();
    assert!(
        answers.contains(&PromptAction::Confirm) && answers.contains(&PromptAction::SkipTurn),
        "and it stands between the two that are answers: {answers:?}"
    );
}
