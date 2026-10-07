//! The burger menu: what it offers and how its panel comes and goes.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// Once a result is standing, this overlay offers nothing.
///
/// Four things, and only the first of them was ever silenced. A refusal
/// or a word about the connection each draws the whole slip on its own,
/// under the end screen and in the veil, saying something about a game
/// that has stopped being played — and the draw and concede controls
/// stayed lit, hovering under the pointer and answering nothing, because
/// `DuelSet::Input` does not run in `Finished`.
///
/// Those two are drawn by `ledge::ways_out` now rather than in the corner
/// up here, and the test did not move with them: `said` reads the whole
/// world, so it is about what the player can see and never about which
/// function put it there.
#[test]
fn nothing_the_overlay_offers_outlives_the_game() {
    let mut app = bar_of(duel_with(true));
    let lines = said(&mut app);
    assert!(
        !lines.iter().any(|l| l.contains(REFUSED)),
        "a refusal outlived the game it refused: {lines:?}"
    );
    let note = Phrase::LinkLost.text(Lang::En).to_string();
    assert!(
        !lines.iter().any(|l| l.contains(&note)),
        "the table is gone and so is the reason to say so: {lines:?}"
    );
    for pill in [Phrase::OfferADraw, Phrase::Concede] {
        let label = pill.text(Lang::En).to_string();
        assert!(
            !lines.contains(&label),
            "a way to end a game that has ended, lit and unanswerable: \
             {lines:?}"
        );
    }
}

/// The counter-test, without which the one above passes on an empty tree.
///
/// It is not a formality here: the first draft of this harness built no
/// tree at all — `sync_overlay` returns early without a board — and the
/// test above passed on the empty world.
#[test]
fn a_game_still_being_played_is_offered_all_four() {
    let mut app = bar_of(duel_with(false));
    let lines = said(&mut app);
    let note = Phrase::LinkLost.text(Lang::En).to_string();
    assert!(
        lines.iter().any(|l| l.contains(&note)),
        "a table that cannot hear you has to say so: {lines:?}"
    );
    // The refusal is there too, and it is the only sentence the shelf
    // draws once the socket is back: one line, and the more urgent of the
    // two takes it.
    let mut reachable = bar_of(duel_saying(false, false));
    let lines = said(&mut reachable);
    assert!(
        lines.iter().any(|l| l.contains(REFUSED)),
        "an answer the engine turned down has to be readable: {lines:?}"
    );
    // The two ways out are behind the burger now, so the shelf carries
    // the mark and not the words. Both halves are asserted: a door that
    // is drawn, and nothing of what is behind it spelled out beside it.
    assert!(
        lines.contains(&glyph::BARS.to_string()),
        "a game still being played has a way out to reach for: {lines:?}"
    );
    for pill in [Phrase::OfferADraw, Phrase::Concede] {
        let label = pill.text(Lang::En).to_string();
        assert!(
            !lines.contains(&label),
            "a shut menu says nothing of what is in it: {lines:?}"
        );
    }
    // And opened, it says both. Through the same `MenuAction` a click
    // sends, rather than by writing the flag, so the test cannot pass on
    // a panel no button can reach — [[client-tests-must-answer-like-a-
    // player]], one level down from a `PlayerAction`.
    crate::input::menu_click(
        &mut reachable.world_mut().resource_mut::<Duel>(),
        MenuAction::ToggleGameMenu,
        false,
    );
    reachable.update();
    let lines = said(&mut reachable);
    for pill in [Phrase::OfferADraw, Phrase::Concede] {
        let label = pill.text(Lang::En).to_string();
        assert!(
            lines.contains(&label),
            "every game still being played offers both of these: {lines:?}"
        );
    }
    assert!(
        lines.iter().any(|l| l.contains(baylee_build::short())),
        "and says which baylee it is: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l == Phrase::ScryfallCredit.text(Lang::En)),
        "and credits Scryfall, whose images the table shows (#325): {lines:?}"
    );
}

/// A refusal this client wrote is read in the player's language, and one
/// another process sent is read in its own.
///
/// The joint, and it had no test: `Phrase` was checked for having a
/// German arm (`i18n`'s two suite-wide tests) and the shelf was checked
/// for drawing *a* refusal (`a_game_still_being_played_is_offered_all_
/// four`), and the line between them — that the sentence reaching the
/// player is the translated one — was asserted by nothing. That is the
/// "declared but never wired" shape this crate keeps finding, and here
/// it had shipped: `Duel::last_error` was a `String`, so nine sentences
/// this client writes were English on a German screen (#121).
///
/// Both arms in one test on purpose. Asserting only the German half
/// would pass on a client that translated *everything* in that slot,
/// which is the opposite defect and the one that would silently rewrite
/// an engine's refusal into a sentence this client made up.
#[test]
fn refusals_follow_the_selected_client_language() {
    // `mine` and not `said`, which is the function three lines down that
    // reads the screen.
    for (lang, mine) in [
        (Lang::De, Phrase::DeedWithdrawn.text(Lang::De)),
        (Lang::En, Phrase::DeedWithdrawn.text(Lang::En)),
    ] {
        // `duel_saying(_, false)` and never `duel_with`: a lost socket
        // takes the one sentence the shelf draws (AX §6), and this test
        // is about what is written in it.
        let mut duel = duel_saying(false, false);
        duel.last_error = Some(baylee_client_core::i18n::Refusal::Said(
            Phrase::DeedWithdrawn,
        ));
        let mut app = bar_of(duel);
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .lang = lang.code().to_string();
        app.update();
        let lines = said(&mut app);
        assert!(
            lines.iter().any(|l| l.contains(mine)),
            "a sentence this client wrote is read in {lang:?}: {lines:?}"
        );
        let other = Phrase::DeedWithdrawn.text(match lang {
            Lang::De => Lang::En,
            Lang::En => Lang::De,
        });
        assert!(
            !lines.iter().any(|l| l.contains(other)),
            "and only in {lang:?} — the other language is on the shelf \
             too: {lines:?}"
        );
    }

    // Known engine refusals follow the selected language as well.
    let mut app = bar_of(duel_saying(false, false));
    app.world_mut()
        .resource_mut::<crate::settings::ClientSettings>()
        .lang = Lang::De.code().to_string();
    app.update();
    let lines = said(&mut app);
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&baylee_client_core::i18n::server_message(Lang::De, REFUSED))),
        "known engine refusals are localized: {lines:?}"
    );
}

/// The panel, its entity and whether it is on the screen.
fn menu_panel(app: &mut App) -> Option<(Entity, bool, usize)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &Visibility, Option<&Children>), With<ledge::menu::MenuPanel>>();
    q.iter(app.world()).next().map(|(e, seen, kids)| {
        (
            e,
            *seen != Visibility::Hidden,
            kids.map_or(0, bevy::ecs::hierarchy::Children::len),
        )
    })
}

/// The scale the panel is drawn at, and where that scale is anchored.
fn menu_pose(app: &mut App) -> (Vec2, Val2) {
    let mut q = app
        .world_mut()
        .query_filtered::<&UiTransform, With<ledge::menu::MenuPanel>>();
    let at = q.single(app.world()).expect("one panel");
    (at.scale, at.translation)
}

/// **The whole reason the panel is not a child of the shelf.**
///
/// Arming the concession writes `LedgeRevision::concede_armed`, which
/// despawns and rebuilds the shelf's columns. The second press has to be
/// made in the panel, so the panel has to be the *same entity* on the
/// other side of that rebuild — and it has to still be up, with its rows
/// redrawn to the confirm wording rather than left saying what they said
/// before.
///
/// Three claims and the first is the one a design that got this wrong
/// would fail: same entity, still shown, and the confirm wording drawn.
#[test]
fn the_panel_survives_the_rebuild_the_arming_press_causes() {
    let mut app = bar_of(duel_with(false));
    crate::input::menu_click(
        &mut app.world_mut().resource_mut::<Duel>(),
        MenuAction::ToggleGameMenu,
        false,
    );
    app.update();
    let (panel, shown, rows) = menu_panel(&mut app).expect("a panel");
    assert!(shown, "the menu is open");
    assert_eq!(
        rows, 8,
        "two ways out, the report row, music controls, the priority sound's \
         switch, a rule, the version and Scryfall's attribution"
    );
    // The shelf's own children, less the two casts: those are spawned
    // with the shelf and exempt from its rebuild, so counting them would
    // make "everything was rebuilt" false on a shelf that rebuilt
    // everything it rebuilds.
    let columns = |app: &mut App| {
        let casts = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<ledge::LedgeCast>>();
            q.iter(app.world()).collect::<Vec<_>>()
        };
        let mut q = app
            .world_mut()
            .query_filtered::<&Children, With<ledge::LedgeShelf>>();
        q.iter(app.world())
            .flat_map(|c| c.iter().collect::<Vec<_>>())
            .filter(|e| !casts.contains(e))
            .collect::<Vec<_>>()
    };
    let before = columns(&mut app);

    // The arming press, through the same door a click uses.
    crate::input::menu_click(
        &mut app.world_mut().resource_mut::<Duel>(),
        MenuAction::Concede,
        false,
    );
    assert!(app.world().resource::<Duel>().concede_armed, "armed");
    app.update();

    // The premise, proved rather than assumed: this test says nothing at
    // all if the shelf did not in fact rebuild, and a shelf that stopped
    // rebuilding would let every claim below pass for the wrong reason.
    let after = columns(&mut app);
    assert!(
        after.iter().all(|e| !before.contains(e)),
        "the arming press did not rebuild the shelf, so this test is not \
         about anything: {before:?} then {after:?}"
    );

    let (again, still, rows_now) = menu_panel(&mut app).expect("still a panel");
    assert_eq!(
        panel, again,
        "the panel the second press has to be made in was despawned by the \
         first press"
    );
    assert!(still, "and it is still on the screen");
    assert_eq!(
        rows_now, rows,
        "and nothing left the column, so the confirm row did not move up \
         under the pointer that is about to press it"
    );
    let lines = said(&mut app);
    let confirm = Phrase::ConcedeConfirm.text(Lang::En).to_string();
    assert!(
        lines.contains(&confirm),
        "and it now asks for the second press: {lines:?}"
    );
    // The draw offer keeps its place and loses its handle, which is what
    // `Weight::Dead` is: drawn, and not a control.
    let offered = {
        let mut q = app.world_mut().query::<&MenuButton>();
        q.iter(app.world())
            .filter(|b| b.action == MenuAction::OfferDraw)
            .count()
    };
    assert_eq!(
        offered, 0,
        "a draw cannot be offered in the middle of conceding, and a row \
         that answers nothing must not be pressable"
    );
}

/// The panel grows out of the shelf's right corner rather than appearing.
///
/// Three claims, and the corner is the one a plain [`motion::from_bottom`]
/// would break: on the frame the menu opens the panel is drawn at
/// [`motion::ZOOM_FROM`], pinned at its **bottom-right** — because a node
/// fixed at the right margin that shrinks toward its own middle slides
/// left as it grows, away from the button that opened it. Then the
/// movement ends at full size.
///
/// With `reduce_motion` it is at full size on that same first frame,
/// which is the counter-test for the first claim on its own terms.
#[test]
fn the_panel_grows_out_of_the_corner_its_button_is_in() {
    for (still, want) in [(false, motion::ZOOM_FROM), (true, 1.0)] {
        let mut app = bar_of(duel_with(false));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = still;
        assert!(
            !menu_panel(&mut app).expect("a panel").1,
            "a menu nobody opened is not on the screen"
        );

        crate::input::menu_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::ToggleGameMenu,
            false,
        );
        app.update();

        assert!(menu_panel(&mut app).expect("a panel").1, "and now it is");
        let (scale, shift) = menu_pose(&mut app);
        assert!(
            (scale.x - want).abs() < 0.001 && (scale.y - want).abs() < 0.001,
            "reduce_motion {still}: drawn at {scale:?}, {want} wanted"
        );
        if !still {
            assert_eq!(
                shift,
                motion::from_bottom_right(want),
                "the panel grows out of the corner its button is in"
            );
            tick(&mut app, motion::ZOOM_IN + 0.01);
            let (scale, shift) = menu_pose(&mut app);
            assert!(
                (scale.x - 1.0).abs() < 0.001,
                "and the arrival ends at full size, not at {scale:?}"
            );
            assert_eq!(shift, motion::from_bottom_right(1.0));
        }
    }
}

/// And it folds back into the shelf rather than being taken off it.
///
/// The clock is driven for the reason
/// `the_strip_folds_back_into_the_shelf_rather_than_being_taken_off_it`
/// records: `bar_of` has no running time, so a test written against the
/// still clock would read the frame the fold *starts* as the frame it
/// ends and would pass against a `grow_the_menu` that hid the panel
/// outright.
#[test]
fn the_panel_folds_away_rather_than_being_taken_away() {
    let mut app = bar_of(duel_with(false));
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;
    crate::input::menu_click(
        &mut app.world_mut().resource_mut::<Duel>(),
        MenuAction::ToggleGameMenu,
        false,
    );
    app.update();
    assert!(menu_panel(&mut app).expect("a panel").1, "open");

    crate::input::menu_click(
        &mut app.world_mut().resource_mut::<Duel>(),
        MenuAction::ToggleGameMenu,
        false,
    );
    tick(&mut app, 0.0);
    assert!(
        menu_panel(&mut app).expect("a panel").1,
        "the panel folds away rather than being taken away"
    );
    let (scale, _) = menu_pose(&mut app);
    assert!(
        (scale.x - 1.0).abs() < 0.001,
        "and the fold begins at full size, not at {scale:?}"
    );

    tick(&mut app, motion::ZOOM_OUT + 0.01);
    assert!(
        !menu_panel(&mut app).expect("a panel").1,
        "and once the fold is over it is away"
    );
}

/// And a game that has ended offers neither, even asked directly.
///
/// `nothing_the_overlay_offers_outlives_the_game`'s other half now that
/// the pair is behind a door: that test reads what is *drawn*, and a shut
/// menu draws nothing either way, so on its own it would pass over a
/// panel that still filled itself with two unanswerable buttons. This one
/// opens it. `DuelSet::Input` does not run in `Finished`, so anything
/// drawn in there would warm under the pointer and answer nothing —
/// exactly what the pair used to do in the corner.
#[test]
fn the_menu_offers_no_way_out_of_a_game_that_has_ended() {
    let mut app = bar_of(duel_with(true));
    app.world_mut().resource_mut::<Duel>().game_menu = true;
    app.update();
    let lines = said(&mut app);
    for pill in [Phrase::OfferADraw, Phrase::Concede, Phrase::ConcedeConfirm] {
        let label = pill.text(Lang::En).to_string();
        assert!(
            !lines.contains(&label),
            "a way to end a game that has ended, opened and unanswerable: \
             {lines:?}"
        );
    }
    assert!(
        !lines.contains(&glyph::BARS.to_string()),
        "and the door itself is gone with the column it stood in: {lines:?}"
    );
}
