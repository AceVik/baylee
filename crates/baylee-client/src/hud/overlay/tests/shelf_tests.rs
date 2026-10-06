//! What outlives a rebuild of the shelf, and the zone dialog.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// Everything the root carries **except** the nodes the sweep is told to
/// pass over.
///
/// Naming the entities is the honest way to say "what the rebuild
/// rebuilds": it means the nodes it lists rather than resting on a
/// component that happens to be drawn in one place — `MenuButton` was
/// that component and stopped being it the moment the ways out of a game
/// moved to the shelf.
///
/// Five now, not two: the zone dialog's veil and panel joined the list
/// when the dialog got a revision of its own, and the tray strip when the
/// dialog gained somewhere to be put down.
///
/// This list and `sync_overlay`'s are hand-kept and separate, which is a
/// drift waiting to happen — and it did, on the commit that added the
/// strip. Catching it is the point; that the failure arrives *here* and
/// not at the sweep is why the caller's assertion has to say both things
/// it can mean.
fn nodes_the_rebuild_rebuilds(app: &mut App) -> Vec<Entity> {
    let kept = {
        let mut q = app.world_mut().query_filtered::<Entity, Or<(
            With<ledge::LedgeShelf>,
            With<ledge::drawer::DrawerRoot>,
            With<TableVeil>,
            With<TrayBand>,
            With<ledge::tray::TrayStrip>,
            With<ledge::pool::PoolStrip>,
            With<ledge::players::PlayersStrip>,
            With<ledge::menu::MenuPanel>,
            With<ledge::log::LogPanel>,
            With<ledge::ai_log::AiLogPanel>,
        )>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    let mut q = app.world_mut().query_filtered::<&Children, With<HudRoot>>();
    q.iter(app.world())
        .flat_map(|c| c.iter().collect::<Vec<_>>())
        .filter(|e| !kept.contains(e))
        .collect::<Vec<_>>()
}

/// The shelf is built once and stands; everything else on the overlay is
/// a picture of the snapshot and is drawn again.
///
/// [`HudRevision`] counts the hover, so this rebuild happens hundreds of
/// times a turn — every time the pointer crosses a card. The shelf is the
/// one node that must survive it: a `Feel`'s warmth is state on the
/// button entity, so a shelf torn down under the pointer snaps the button
/// the player is reaching for back to rest, and the ledge's own revision
/// counter would govern a subtree that is deleted before it can be
/// compared.
///
/// The counter-half of the assertion is the important one. Without it
/// this passes on an overlay that stopped rebuilding at all, which is the
/// much worse bug of the two: a bar that never redraws says the wrong
/// thing about the game for as long as the game lasts.
#[test]
#[allow(clippy::too_many_lines)] // five queries, each read twice
fn the_shelf_and_its_attachments_outlive_a_rebuild_and_nothing_else_does() {
    // A print table, because the counter-half needs something the overlay
    // *does* draw under the root, and everything it draws there asks for
    // one. The two ways out of a game were the exception — spawned with
    // nothing but a language — and they are on the shelf now (AX §4.3),
    // so what answers for the overlay is the hand zone.
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    let mut app = bar_of(duel);

    let shelf = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::LedgeShelf>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    let roots = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    // What the shelf carries, and what the overlay carries: the two sides
    // of the claim.
    //
    // The shelf's two casts are not columns and are filtered out: they
    // are spawned with the shelf and exempt from the rebuild, so counting
    // them here would make "three columns" read five and would let a
    // column that stopped being built pass unnoticed.
    let standing = |app: &mut App| {
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
    let drawer = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::drawer::DrawerRoot>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    // The three strips beside them, each surviving for an argument of
    // its own — `OverlayTree`'s own fields say which. They are read as
    // one list because the *claim* is identical for all three and the
    // sweep is one condition per strip, so a fourth attachment costs one
    // line here rather than a paragraph, and the message still names
    // which of them went.
    //
    // They are checked by **identity** because the sweep does not leave a
    // hole behind it: a strip despawned here is spawned again on the same
    // frame, by the branch below that builds the root. So the count is
    // one either way, the picture is right on the next frame, and what is
    // actually lost is the `MenuZoom` or the `PoolReveal` that was on the
    // old entity — which is why removing any one of those three
    // conditions from the sweep passed every test in this file.
    let strips = |app: &mut App| {
        let mut tray = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::tray::TrayStrip>>();
        let tray = tray.iter(app.world()).collect::<Vec<_>>();
        let mut pool = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::pool::PoolStrip>>();
        let pool = pool.iter(app.world()).collect::<Vec<_>>();
        let mut players = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::players::PlayersStrip>>();
        let players = players.iter(app.world()).collect::<Vec<_>>();
        let mut menu = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::menu::MenuPanel>>();
        let menu = menu.iter(app.world()).collect::<Vec<_>>();
        let mut log = app
            .world_mut()
            .query_filtered::<Entity, With<ledge::log::LogPanel>>();
        let log = log.iter(app.world()).collect::<Vec<_>>();
        [
            ("tray", tray),
            ("mana pool", pool),
            ("players' strip", players),
            ("game menu", menu),
            ("game log", log),
        ]
    };

    let was_shelf = shelf(&mut app);
    let was_drawer = drawer(&mut app);
    let was_root = roots(&mut app);
    let was_standing = standing(&mut app);
    let was_strips = strips(&mut app);
    let was_redrawn = nodes_the_rebuild_rebuilds(&mut app);
    assert_eq!(was_shelf.len(), 1, "one shelf, and it was built");
    assert_eq!(was_drawer.len(), 1, "and one drawer beside it");
    assert_eq!(was_root.len(), 1, "and one root to hang them off");
    assert_eq!(
        was_standing.len(),
        3,
        "hand tools, answers, and game controls — the mana pool left this \
         node for a strip of its own"
    );
    assert!(
        !was_redrawn.is_empty(),
        "the overlay drew something of its own beside the two"
    );
    for (name, found) in &was_strips {
        assert_eq!(found.len(), 1, "one {name}, and it was built");
    }

    // The pointer moves onto a card. Nothing about the game changed.
    app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
    app.update();

    assert_eq!(shelf(&mut app), was_shelf, "the shelf was rebuilt");
    assert_eq!(
        drawer(&mut app),
        was_drawer,
        "the drawer's node was rebuilt, and a panel open under the \
         pointer would have gone with it"
    );
    assert_eq!(roots(&mut app), was_root, "and so was the root under it");
    let now_strips = strips(&mut app);
    for ((name, was), (_, now)) in was_strips.iter().zip(now_strips.iter()) {
        assert_eq!(
            was, now,
            "the {name} was taken off the root and built again. Nothing                  else can do that — it is spawned once, with the root — so                  the sweep above has stopped sparing it, and whatever the                  strip was in the middle of has been thrown away with the                  entity that was doing it"
        );
    }
    assert_eq!(
        standing(&mut app),
        was_standing,
        "the shelf kept its place and lost what was on it, which is the \
         same loss one level down: a `Feel` under the pointer goes back \
         to rest"
    );
    let now_redrawn = nodes_the_rebuild_rebuilds(&mut app);
    assert!(!now_redrawn.is_empty(), "the overlay still draws it");
    // The count is what separates the two things this can mean, so it is
    // in the message: *all* of them surviving is a rebuild that stopped,
    // and one of them is a new attachment beside the shelf that `kept`
    // above has not been told about.
    let stale = now_redrawn
        .iter()
        .filter(|e| was_redrawn.contains(e))
        .count();
    assert!(
        stale == 0,
        "{stale} of {} nodes under the root came through the rebuild \
         unchanged. All of them means the overlay stopped rebuilding and \
         is showing the tree it built for a different frame; one or two \
         means something new stands beside the shelf and this test's \
         `kept` list has not been told — `sync_overlay`'s own sweep is \
         the list to hold it against.",
        now_redrawn.len()
    );
}

/// The owner's report, as an assertion: *„Das Zonen-Dialog ist noch sehr
/// instabil! Beim Hover flackert alles"*.
///
/// The dialog is a hundred rows and the pointer moves *across* them, so
/// every row it reached tore the whole overlay down and wrote it again —
/// the dialog with it, because the dialog was part of that tree. What the
/// player sees is the row they are reaching for going out: the
/// replacement is a new entity with a fresh [`Feel`] at `warmth: 0`, and
/// picking needs a frame to send `Over` to something that did not exist
/// when it last looked.
///
/// Both halves, and the second is the one that makes the first mean
/// anything: a dialog that had simply stopped being drawn would pass the
/// first assertion perfectly. So the filter is typed into next, which is
/// a change the dialog *must* answer, and the same entity standing there
/// would be the opposite defect — a panel showing a list nobody narrowed.
#[test]
fn the_zone_dialog_outlives_a_pointer_move_and_not_a_search() {
    let mut duel = duel_with(false);
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    duel.browser.open();
    let mut app = bar_of(duel);

    let panel = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<TrayBand>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    let veil = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<TableVeil>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };
    // The overlay's own half, so this cannot pass on a renderer that has
    // stopped rebuilding anything at all.
    let hand = |app: &mut App| {
        let mut q = app.world_mut().query_filtered::<Entity, With<HandScroll>>();
        q.iter(app.world()).collect::<Vec<_>>()
    };

    let was_panel = panel(&mut app);
    let was_veil = veil(&mut app);
    let was_hand = hand(&mut app);
    assert_eq!(was_panel.len(), 1, "the dialog was drawn at all");
    assert_eq!(was_veil.len(), 1, "and the veil behind it");
    assert_eq!(was_hand.len(), 1, "and the overlay drew its own hand zone");

    // The pointer moves onto a card. Nothing about the dialog changed.
    app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
    app.update();

    assert_eq!(panel(&mut app), was_panel, "the dialog was rebuilt");
    assert_eq!(veil(&mut app), was_veil, "and so was the veil behind it");
    assert!(
        hand(&mut app).iter().all(|e| !was_hand.contains(e)),
        "the overlay stopped rebuilding, so the dialog standing still \
         says nothing about the dialog"
    );

    // And the counter-half: a letter in the filter box is a different
    // list, and a different list is a rebuild.
    app.world_mut()
        .resource_mut::<Duel>()
        .browser
        .push_filter('a');
    app.update();
    assert!(
        panel(&mut app).iter().all(|e| !was_panel.contains(e)),
        "the search narrowed nothing: the dialog is showing the list it \
         built before the letter was typed"
    );
}

/// The owner's other complaint about this dialog: a row grew under the
/// pointer.
///
/// A row is a **line of writing** — a tick, a picture, a name, the pips,
/// a type line — and [`Feel::lift`]'s own doc names that as the case that
/// must stay at zero: growing a row grows the sentence on it, and a
/// hundred of them under a moving pointer is a list that reflows while it
/// is being read. `row_slot` carries `Feel::tinting_to` for exactly that
/// reason, which lights the row and leaves it where it is.
///
/// Both halves, and the second is what makes the first mean anything: a
/// query that found no rows, or a dialog in which nothing lifts at all,
/// would pass "every row is at zero" perfectly. So the same question is
/// put to the **grid**, where a tile is a picture and answers the pointer
/// by growing — the one place this dialog parts company with itself, and
/// it is deliberate.
///
/// Both list modes, because `Large` is the detailed row with its asides
/// dropped and its picture doubled: it is the same `row_slot` and it is
/// the mode a second hand would forget.
#[test]
fn a_row_is_lit_under_the_pointer_and_never_grows() {
    use baylee_client_core::browser::ViewMode;

    fn lifts(app: &mut App) -> Vec<f32> {
        let mut found = app
            .world_mut()
            .query_filtered::<&crate::ambience::Feel, With<TrayCard>>();
        found.iter(app.world()).map(|feel| feel.lift).collect()
    }

    fn dialog_in(mode: ViewMode) -> App {
        // A row and a tile both draw a thumbnail, and asking for one is
        // an `AssetServer::load` — which spawns on the IO pool and panics
        // without it. Idempotent, so several tests may ask.
        bevy::tasks::IoTaskPool::get_or_init(Default::default);
        let mut duel = duel_with(false);
        duel.statics = Some(baylee_client_core::test_support::statics(8));
        duel.view = Some(
            baylee_client_core::test_support::ViewBuilder::new(2)
                .with_graveyard(
                    0,
                    (10..16)
                        .map(|s| baylee_client_core::test_support::printed(s, 0, "Forest", 1))
                        .collect(),
                )
                .build(),
        );
        crate::rebuild_board(&mut duel);
        duel.browser.open();
        let mut app = bar_of(duel);
        // After the harness has run once, so this is the change that
        // rebuilds: the view mode is in `TrayRevision`'s browser gate.
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .zone_view = mode;
        app.update();
        app
    }

    for mode in [ViewMode::Detailed, ViewMode::Large] {
        let mut app = dialog_in(mode);
        let rows = lifts(&mut app);
        assert_eq!(
            rows.len(),
            6,
            "{mode:?}: the list drew a row per card in the pile, and did \
             not: {rows:?}"
        );
        assert!(
            rows.iter().all(|lift| *lift == 0.0),
            "{mode:?}: a row lifts, so it grows under the pointer and the \
             sentence on it grows with it: {rows:?}"
        );
    }

    let mut app = dialog_in(ViewMode::Grid);
    let tiles = lifts(&mut app);
    assert_eq!(tiles.len(), 6, "the grid drew a tile per card: {tiles:?}");
    assert!(
        tiles.iter().all(|lift| *lift > 0.0),
        "nothing in this dialog lifts at all, so the rows standing at zero \
         says nothing about the rows: {tiles:?}"
    );
}
