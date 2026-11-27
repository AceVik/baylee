//! Importing and exporting a deck, through the controls a player has: the
//! deck list's Import button opens the builder with the import dialog in
//! front of it, what is pasted is read and reported, Import deck takes it,
//! and the export dialog offers every format with its copy and its keys.
//! What the formats say is `baylee-deckio`'s; what the builder makes of it,
//! client-core's `transfer_tests`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_client_core::deckbuilder::transfer::{FormatId, Transfer};

fn transfer(app: &App) -> Option<Transfer> {
    app.world()
        .resource::<LobbyState>()
        .lobby
        .builder()
        .transfer()
        .cloned()
}

/// Presses one key for one frame, and lets it go: in `ButtonInput`, and as
/// the event the shell keymap reads (a letter as its character).
fn key(app: &mut App, held: Option<KeyCode>, code: KeyCode) {
    {
        let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if let Some(held) = held {
            codes.press(held);
        }
        codes.press(code);
    }
    let name = format!("{code:?}");
    let letter = name
        .strip_prefix("Key")
        .filter(|l| l.len() == 1)
        .map(str::to_lowercase);
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write(KeyboardInput {
            key_code: code,
            logical_key: letter.clone().map_or(
                Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                |l| Key::Character(l.into()),
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    codes.release_all();
    codes.clear();
}

#[test]
fn a_pasted_list_is_imported_from_the_deck_list_and_reported() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    app.update();
    to_decks(&mut app);
    press(&mut app, Press::Decks(DecksPress::Import));
    assert!(
        matches!(transfer(&app), Some(Transfer::Import(_))),
        "the deck list's Import opens the builder on the import dialog"
    );
    let found = presses(&mut app);
    for wanted in [
        Press::Build(BuildPress::ImportPaste),
        Press::Build(BuildPress::TransferClose),
    ] {
        assert!(found.contains(&wanted), "{wanted:?} missing from {found:?}");
    }
    assert!(
        !found.contains(&Press::Build(BuildPress::ImportTake)),
        "nothing pasted, nothing to take: {found:?}"
    );

    // What the clipboard would hand over; the clipboard itself is the
    // platform's, and this app has none.
    app.world_mut()
        .resource_mut::<LobbyState>()
        .lobby
        .builder_mut()
        .import_paste("4 Llanowar Elves\n20 Forest\n1 Nowhere Card\n");
    app.update();
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l.contains("4 Llanowar Elves")),
        "the pasted text is shown: {drawn:?}"
    );
    press(&mut app, Press::Build(BuildPress::ImportTake));
    let state = app.world().resource::<LobbyState>();
    let deck = state.lobby.builder();
    assert_eq!(deck.counts().main, 24, "both known cards were taken");
    assert_eq!(deck.missing(), ["Nowhere Card".to_string()]);
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l.contains("Nowhere Card")),
        "the unknown card is named, not dropped: {drawn:?}"
    );
    // The finished import's one way on is Close, and Enter presses it.
    key(&mut app, None, KeyCode::Enter);
    assert!(transfer(&app).is_none(), "Enter closed the finished import");
}

#[test]
fn the_export_dialog_offers_every_format_and_answers_its_keys() {
    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1400.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let builder = state.lobby.builder_mut();
        builder.set_name("Elves");
        assert!(builder.add(0, Zone::Main), "the pool has that card");
    }
    app.update();
    // The builder's own chord opens it, on every platform: the shell
    // keymap's Export (Cmd on macOS, Ctrl elsewhere).
    let command = if crate::shellkit::keys::mac() {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    };
    key(&mut app, Some(command), KeyCode::KeyE);
    assert!(
        matches!(transfer(&app), Some(Transfer::Export(_))),
        "Ctrl+E opened the export dialog"
    );
    let found = presses(&mut app);
    for format in FormatId::ALL {
        assert!(
            found.contains(&Press::Build(BuildPress::ExportFormat(format))),
            "{format:?} missing from {found:?}"
        );
    }
    assert!(
        found.contains(&Press::Build(BuildPress::ExportCopy)),
        "{found:?}"
    );

    press(
        &mut app,
        Press::Build(BuildPress::ExportFormat(FormatId::Json)),
    );
    let drawn = labels(&mut app);
    assert!(
        drawn.iter().any(|l| l.contains("\"version\": 1")),
        "the chosen format is previewed: {drawn:?}"
    );
    key(&mut app, None, KeyCode::ArrowRight);
    let Some(Transfer::Export(open)) = transfer(&app) else {
        panic!("the dialog stays open");
    };
    assert_ne!(open.format, FormatId::Json, "→ steps to the next format");
    key(&mut app, None, KeyCode::Escape);
    assert!(transfer(&app).is_none(), "Esc closes the dialog");
}

/// Both Export and Import preview boxes keep long text inside a clipping,
/// non-shrinking border with a scrolling viewport so lines never spill over
/// the summary or shortcut rows below.
#[test]
fn transfer_preview_boxes_scroll_inside_a_clipping_frame() {
    let assert_bounded = |app: &mut App, needle: &str| {
        let text_entity = {
            let world = app.world_mut();
            world
                .query::<(Entity, &Text)>()
                .iter(world)
                .find(|(_, t)| t.0.contains(needle))
                .map_or_else(|| panic!("preview text containing {needle:?}"), |(e, _)| e)
        };
        let mut cur = app.world().get::<ChildOf>(text_entity).map(ChildOf::parent);
        let mut saw_scroller = false;
        let mut saw_frame = false;
        while let Some(e) = cur {
            if app
                .world()
                .get::<crate::shellkit::surfaces::SheetScroller>(e)
                .is_some()
                && let Some(node) = app.world().get::<Node>(e)
                && node.overflow.y == bevy::ui::OverflowAxis::Scroll
            {
                saw_scroller = true;
            }
            if saw_scroller
                && let Some(node) = app.world().get::<Node>(e)
                && node.flex_shrink == 0.0
                && node.overflow == Overflow::clip()
                && matches!(node.max_height, Val::Px(h) if h > 0.0)
            {
                saw_frame = true;
                break;
            }
            cur = app.world().get::<ChildOf>(e).map(ChildOf::parent);
        }
        assert!(
            saw_scroller,
            "preview text lives inside a vertical scroller"
        );
        assert!(
            saw_frame,
            "preview scroller sits inside a non-shrinking, bounded, clipping frame"
        );
    };

    let mut app = headless();
    stocked(&mut app);
    sized(&mut app, 1280.0);
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.build_deck();
        let builder = state.lobby.builder_mut();
        builder.set_name("Elves");
        assert!(builder.add(0, Zone::Main));
        builder.open_export();
    }
    app.update();
    press(
        &mut app,
        Press::Build(BuildPress::ExportFormat(FormatId::Json)),
    );
    assert_bounded(&mut app, "\"version\": 1");

    press(&mut app, Press::Build(BuildPress::TransferClose));
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        let builder = state.lobby.builder_mut();
        builder.open_import();
        let long_list = (0..50)
            .map(|i| format!("1 Card {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        builder.import_paste(&long_list);
    }
    app.update();
    assert_bounded(&mut app, "1 Card 0");
}
