//! The deck builder's acceptance on the shell kit (the shell design §7,
//! §17 WP4): one save state, no grid switch, Esc that never leaves, a typed
//! key that patches and never rebuilds, the shapes by size class, Shift on
//! a pool row's `+`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::buildui::{DeckTab, Nav, Pane};

/// A window of a given size, so the phone's raw height can be reached.
fn window(app: &mut App, width: f32, height: f32) {
    let mut existing = app.world_mut().query::<&mut Window>();
    if let Some(mut window) = existing.iter_mut(app.world_mut()).next() {
        window.resolution.set(width, height);
        return;
    }
    let mut window = Window::default();
    window.resolution.set(width, height);
    app.world_mut().spawn(window);
}

/// The builder open on a new deck at a window size, the deck holding a card.
fn builder_at(width: f32, height: f32) -> App {
    let mut app = headless();
    stocked(&mut app);
    window(&mut app, width, height);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.build = crate::buildui::BuildUi::opened();
        state.lobby.build_deck();
        state.lobby.builder_mut().add(0, Zone::Main);
    }
    for _ in 0..3 {
        app.update();
    }
    app
}

fn save_states(app: &mut App) -> usize {
    app.world_mut()
        .query::<&crate::buildui::header::SaveStateNode>()
        .iter(app.world())
        .count()
}

fn rebuilds(app: &App) -> UiRebuilds {
    *app.world().resource::<UiRebuilds>()
}

fn key(app: &mut App, input: KeyboardInput) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(input);
    app.update();
}

/// One save state on the screen (§2.5: "one save-state element per screen,
/// the builder's is in the header"), at every shape and through a save.
#[test]
fn the_builder_shows_one_save_state() {
    for (w, h) in [
        (1920.0, 1080.0),
        (960.0, 700.0),
        (844.0, 390.0),
        (640.0, 360.0),
    ] {
        let mut app = builder_at(w, h);
        assert_eq!(save_states(&mut app), 1, "{w}×{h}");
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .builder_mut()
            .set_name("Elves");
        app.update();
        assert_eq!(save_states(&mut app), 1, "{w}×{h}, unsaved");
        let said: Vec<String> = labels(&mut app)
            .into_iter()
            .filter(|t| t.starts_with("Unsaved"))
            .collect();
        if w > 900.0 {
            assert_eq!(said, ["Unsaved · 2 changes"], "{w}×{h}");
        }
    }
}

/// The grid view is round 2 and not drawn (S4-1): no `▦`, no view switch.
#[test]
fn no_grid_switch_is_drawn() {
    for (w, h) in [(1920.0, 1080.0), (960.0, 700.0), (844.0, 390.0)] {
        let mut app = builder_at(w, h);
        let grid = crate::hud::glyph::VIEW_GRID.to_string();
        assert!(
            !labels(&mut app)
                .iter()
                .any(|t| t.contains('\u{25a6}') || t.contains(&grid)),
            "{w}×{h}"
        );
    }
}

/// Esc ×10 with an unsaved deck and nothing open: still the builder, the
/// deck unchanged (KEYBOARD §2.5, S4-8). With the search holding words the
/// first Esc clears them; the other nine do nothing.
#[test]
fn escape_ten_times_stays_in_the_builder() {
    let mut app = builder_at(1920.0, 1080.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.builder_mut().set_name("Half a deck");
        state.lobby.builder_mut().set_text("goblin");
        state.build.rail = true;
    }
    app.update();
    let before = app
        .world()
        .resource::<LobbyState>()
        .lobby
        .builder()
        .entries(Zone::Main)
        .to_vec();
    for _ in 0..10 {
        key(&mut app, pressed(KeyCode::Escape, Key::Escape));
    }
    let state = app.world().resource::<LobbyState>();
    assert_eq!(state.lobby.screen(), &Screen::Build, "Esc never leaves");
    assert!(!state.build.rail, "the first Esc closed the rail");
    assert_eq!(
        state.lobby.builder().text(),
        "",
        "the next cleared the search"
    );
    assert_eq!(state.lobby.builder().entries(Zone::Main), before.as_slice());
    assert!(state.lobby.builder().dirty(), "and the deck is as it was");
    assert!(!state.confirm_leave, "nothing asked to leave");
}

/// A key typed into the search patches the builder — its toolbar and its
/// list — and never rebuilds the tree (§10 #1, #2: "1 rebuild per key").
#[test]
fn a_typed_key_is_one_patch_and_no_rebuild() {
    let mut app = builder_at(1920.0, 1080.0);
    let before = rebuilds(&app);
    for ch in "elf".chars() {
        key(&mut app, typed(ch));
    }
    let after = rebuilds(&app);
    assert_eq!(after.total, before.total, "the tree was rebuilt for a key");
    assert_eq!(after.patches - before.patches, 3, "one patch per key");
    assert!(
        after.sections - before.sections <= 3 * 3,
        "a key redraws its toolbar, its list and the count, not the deck: {}",
        after.sections - before.sections
    );
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.builder().text(),
        "elf"
    );
}

/// The shape follows the size class (§2.7): Wide two columns; Narrow one
/// pane and a tab bar; a phone the pool and the deck rail; 640 one pane and
/// a switch in the header.
#[test]
fn each_size_class_has_its_shape() {
    let mut wide = builder_at(1920.0, 1080.0);
    let found = presses(&mut wide);
    assert!(found.contains(&Press::Build(BuildPress::AddFromPool(0, false))));
    assert!(found.contains(&Press::Build(BuildPress::SetTab(DeckTab::Stats))));
    assert!(
        found.contains(&Press::Build(BuildPress::OpenImport)),
        "Import in the header"
    );
    assert!(!found.contains(&Press::Build(BuildPress::SetPane(Pane::Deck))));

    let mut narrow = builder_at(960.0, 700.0);
    let found = presses(&mut narrow);
    assert!(
        found.contains(&Press::Build(BuildPress::SetPane(Pane::Stats))),
        "the tab bar"
    );
    assert!(
        !found.contains(&Press::Build(BuildPress::OpenImport)),
        "Import is in ⋯"
    );

    let mut phone = builder_at(844.0, 390.0);
    let found = presses(&mut phone);
    assert!(
        found.contains(&Press::Build(BuildPress::AddFromPool(0, false))),
        "the pool"
    );
    assert!(
        found.contains(&Press::Build(BuildPress::OpenStatsSheet)),
        "the deck rail"
    );
    assert!(found.contains(&Press::Build(BuildPress::SetTab(DeckTab::Side))));

    let mut small = builder_at(640.0, 360.0);
    let found = presses(&mut small);
    assert!(
        found.contains(&Press::Build(BuildPress::SetPane(Pane::Deck))),
        "the switch"
    );
    assert!(
        !found.contains(&Press::Build(BuildPress::OpenStatsSheet)),
        "no rail"
    );
}

/// Shift on a pool row's `+` adds to the other list (§7: "one `+`
/// (active tab; Shift → other)").
#[test]
fn shift_on_a_pool_plus_adds_to_the_other_list() {
    let mut app = builder_at(1920.0, 1080.0);
    let plus = press_target(&mut app, Press::Build(BuildPress::AddFromPool(1, false)));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    tap(&mut app, plus);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ShiftLeft);
    let deck = app.world().resource::<LobbyState>().lobby.builder();
    assert_eq!(deck.count_of(1, Zone::Side), 1, "to the sideboard");
    assert_eq!(deck.count_of(1, Zone::Main), 0);
    // Without Shift, the list the deck side shows.
    press(&mut app, Press::Build(BuildPress::AddFromPool(1, false)));
    let deck = app.world().resource::<LobbyState>().lobby.builder();
    assert_eq!(deck.count_of(1, Zone::Main), 1);
    // With the sideboard shown, `+` fills it.
    press(&mut app, Press::Build(BuildPress::SetTab(DeckTab::Side)));
    press(&mut app, Press::Build(BuildPress::AddFromPool(1, false)));
    let deck = app.world().resource::<LobbyState>().lobby.builder();
    assert_eq!(deck.count_of(1, Zone::Side), 2);
}

/// The keyboard walks the pool (KEYBOARD §7.7): ↓ from the search onto the
/// first row, Enter adds, ⇧Enter to the other list, ↑ from the first row
/// back to the search.
#[test]
fn the_keyboard_walks_the_pool_and_adds() {
    let mut app = builder_at(1920.0, 1080.0);
    assert_eq!(app.world().resource::<LobbyState>().build.nav, Nav::Field);
    key(&mut app, pressed(KeyCode::ArrowDown, Key::ArrowDown));
    assert_eq!(app.world().resource::<LobbyState>().build.nav, Nav::Pool(0));
    let first = app
        .world()
        .resource::<LobbyState>()
        .lobby
        .builder()
        .results()[0];
    let held = |app: &App, zone| {
        app.world()
            .resource::<LobbyState>()
            .lobby
            .builder()
            .count_of(first, zone)
    };
    let main = held(&app, Zone::Main);
    key(&mut app, pressed(KeyCode::Enter, Key::Enter));
    assert_eq!(held(&app, Zone::Main), main + 1, "Enter added");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, pressed(KeyCode::Enter, Key::Enter));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ShiftLeft);
    assert_eq!(held(&app, Zone::Side), 1, "⇧Enter to the sideboard");
    key(&mut app, pressed(KeyCode::ArrowUp, Key::ArrowUp));
    assert_eq!(
        app.world().resource::<LobbyState>().build.nav,
        Nav::Field,
        "↑ on the first row is the search again"
    );
}

/// F2 renames: the title becomes its box with the name selected, typing
/// replaces it, Enter ends it and frees the keyboard.
#[test]
fn f2_renames_the_deck() {
    let mut app = builder_at(1920.0, 1080.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .set_name("Old");
    app.update();
    key(&mut app, pressed(KeyCode::F2, Key::F2));
    for ch in "Elves".chars() {
        key(&mut app, typed(ch));
    }
    key(&mut app, pressed(KeyCode::Enter, Key::Enter));
    let state = app.world().resource::<LobbyState>();
    assert_eq!(
        state.lobby.builder().name(),
        "Elves",
        "typing replaced the name"
    );
    assert_eq!(state.build.nav, Nav::Idle);
    assert!(
        labels(&mut app).iter().any(|t| t == "Elves"),
        "the title says so"
    );
}

/// The deck side's sections fold and open again, and the grouping changes
/// what they are.
#[test]
fn the_deck_list_groups_and_folds() {
    let mut app = builder_at(1920.0, 1080.0);
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .add(1, Zone::Main);
    app.update();
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
    press(&mut app, Press::Build(BuildPress::CollapseAll));
    assert!(
        !presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))),
        "every section folded"
    );
    press(&mut app, Press::Build(BuildPress::CollapseAll));
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
    press(
        &mut app,
        Press::Build(BuildPress::SetGrouping(
            baylee_client_core::deckbuilder::Grouping::ManaValue,
        )),
    );
    assert_eq!(
        app.world().resource::<LobbyState>().build.grouping,
        baylee_client_core::deckbuilder::Grouping::ManaValue
    );
    assert!(presses(&mut app).contains(&Press::Build(BuildPress::RemoveRow(0))));
}
