//! The updater's face in a headless app: the notice appears where it
//! should, its buttons do what they say, and the switches reach the thread.

use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::picking::events::{Click, Pointer};

fn fonts() -> UiFonts {
    UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    }
}

/// The face alone, as `native` installs it, with no thread behind it.
fn headless() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<DuelPhase>()
        .add_message::<Pointer<Click>>()
        .insert_resource(fonts())
        .add_plugins(UpdatePlugin);
    app.update();
    app
}

fn ready() -> Shown {
    Shown::Ready {
        version: "0.1.0-beta.3".into(),
        page: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into(),
    }
}

fn texts(app: &mut App) -> Vec<String> {
    let mut query = app.world_mut().query::<&Text>();
    query.iter(app.world()).map(|t| t.0.clone()).collect()
}

fn toasts(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<(), With<UpdateToast>>();
    query.iter(app.world()).count()
}

fn button(app: &mut App, wanted: &UpdateButton) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &UpdateButton)>();
    query
        .iter(app.world())
        .find_map(|(e, b)| (b == wanted).then_some(e))
        .unwrap_or_else(|| panic!("no {wanted:?} button"))
}

/// One click, as the picking backend reports it.
fn click(app: &mut App, entity: Entity) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap_or_else(|_| {
            app.world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id()
        });
    let camera = app.world_mut().spawn_empty().id();
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    };
    let event = Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    app.update();
}

fn requests(app: &mut App) -> Vec<UpdateRequest> {
    let mut messages = app.world_mut().resource_mut::<Messages<UpdateRequest>>();
    messages.drain().collect()
}

fn phase(app: &mut App, next: DuelPhase) {
    app.world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(next);
    app.update();
}

#[test]
fn no_notice_draws_nothing() {
    let mut app = headless();
    app.update();
    assert_eq!(toasts(&mut app), 0);
}

#[test]
fn a_staged_update_is_announced_in_the_lobby_and_offered_at_a_table() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(ready());
    app.update();
    assert_eq!(toasts(&mut app), 1);
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "Update 0.1.0-beta.3 ready – installs when you quit"),
        "{:?}",
        texts(&mut app)
    );
    assert!(texts(&mut app).iter().any(|t| t == "Release notes"));

    phase(&mut app, DuelPhase::Playing);
    app.update();
    assert_eq!(toasts(&mut app), 1, "at a table, the restart is offered");
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == Phrase::UpdateRestartOffer.text(Lang::En))
    );

    phase(&mut app, DuelPhase::Closed);
    app.update();
    assert_eq!(toasts(&mut app), 1, "back in the lobby, it is back");
}

/// Only a ready update is offered at a table: a link waits for the lobby
/// (the menu's line says it there).
#[test]
fn a_link_is_not_shown_over_a_table() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Available {
        version: "0.1.0-beta.3".into(),
        page: "https://example.invalid/".into(),
        why: Why::DevBuild,
    });
    phase(&mut app, DuelPhase::Playing);
    app.update();
    assert_eq!(toasts(&mut app), 0);
}

/// The table, with this seat owing the answer to question `seq`, or owing
/// nothing.
fn table(app: &mut App, asked: Option<u64>) {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.awaiting = Some(if asked.is_some() {
        view.seat
    } else {
        baylee_core::ids::PlayerId::new(1)
    });
    view.seq = asked.unwrap_or(view.seq);
    app.world_mut()
        .get_resource_or_insert_with(crate::Duel::default)
        .view = Some(view);
    app.update();
}

/// The restart offer never appears in the middle of a question of mine: an
/// update that is ready while I decide waits for my answer and appears
/// with what comes next, and then stays, a panel and no question. "Restart
/// now" asks for the restart; "Later" puts the offer away for the session.
#[test]
fn the_restart_offer_never_interrupts_my_pending_decision() {
    let mut app = headless();
    phase(&mut app, DuelPhase::Playing);
    table(&mut app, Some(5));
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(ready());
    app.update();
    assert_eq!(toasts(&mut app), 0, "I am deciding: no offer beside it");
    table(&mut app, Some(5));
    assert_eq!(toasts(&mut app), 0, "still the same question");
    assert!(requests(&mut app).is_empty());

    table(&mut app, Some(6));
    assert_eq!(toasts(&mut app), 1, "answered: the offer appears now");
    table(&mut app, None);
    assert_eq!(toasts(&mut app), 1, "and stays");
    let restart = button(&mut app, &UpdateButton::RestartNow);
    click(&mut app, restart);
    assert_eq!(requests(&mut app), [UpdateRequest::RestartNow]);

    let later = button(&mut app, &UpdateButton::Later);
    click(&mut app, later);
    app.update();
    assert_eq!(toasts(&mut app), 0, "put off");
    assert!(requests(&mut app).is_empty(), "later asks for nothing");
    table(&mut app, None);
    assert_eq!(toasts(&mut app), 0, "for the session");
}

#[test]
fn a_link_says_why_it_is_only_a_link() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Available {
        version: "0.1.0-beta.3".into(),
        page: "https://example.invalid/".into(),
        why: Why::DevBuild,
    });
    app.update();
    let lines = texts(&mut app);
    assert!(
        lines
            .iter()
            .any(|t| t == Phrase::UpdateWhyDev.text(Lang::En)),
        "{lines:?}"
    );
}

#[test]
fn later_puts_the_notice_away_and_release_notes_open_the_page() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(ready());
    app.update();
    let notes = button(
        &mut app,
        &UpdateButton::Open("https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into()),
    );
    opened().clear();
    click(&mut app, notes);
    assert_eq!(
        opened().clone(),
        ["https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3"]
    );

    let hide = button(&mut app, &UpdateButton::Later);
    click(&mut app, hide);
    app.update();
    assert!(app.world().resource::<UpdateNotice>().hidden);
    assert_eq!(toasts(&mut app), 0);
}

/// The two toggles' settings, check first.
fn switches(app: &mut App) -> Vec<bool> {
    let mut q = app.world_mut().query::<(&Readout, &Children)>();
    let mut toggles = Vec::new();
    for (readout, children) in q.iter(app.world()) {
        for child in children {
            if let Some(motion) = app
                .world()
                .get::<crate::shellkit::controls::ToggleMotion>(*child)
            {
                toggles.push((*readout == Readout::Install, motion.on));
            }
        }
    }
    toggles.sort_unstable();
    toggles.into_iter().map(|(_, on)| on).collect()
}

#[test]
fn the_switches_flip_this_devices_choice_and_tell_the_thread() {
    let mut app = headless();
    app.world_mut()
        .run_system_once(|mut commands: Commands, fonts: Res<UiFonts>| {
            let kit = crate::shellkit::controls::Kit {
                fonts: &fonts,
                m: crate::shellkit::metrics::ShellMetrics::of(
                    crate::shellkit::size::Viewport::desktop(1280.0, 800.0),
                    crate::shellkit::size::TextSize::M,
                ),
                german: false,
            };
            controls(&mut commands, kit, Metrics::of(1280.0), Lang::En);
        })
        .expect("the controls build");
    app.update();
    assert_eq!(
        *app.world().resource::<UpdatePrefs>(),
        UpdatePrefs::default(),
        "both on by default"
    );
    // The kit's toggles, as every other section draws a switch: no boxed
    // "on" text.
    assert_eq!(switches(&mut app), [true, true]);
    assert!(!texts(&mut app).iter().any(|t| t == "on"));

    let check = button(&mut app, &UpdateButton::ToggleCheck);
    click(&mut app, check);
    let prefs = *app.world().resource::<UpdatePrefs>();
    assert_eq!(
        prefs,
        UpdatePrefs {
            check: false,
            install: true
        }
    );
    assert_eq!(requests(&mut app), [UpdateRequest::Prefs(prefs)]);
    app.update();
    assert_eq!(switches(&mut app), [false, true]);

    let install = button(&mut app, &UpdateButton::ToggleInstall);
    click(&mut app, install);
    let prefs = *app.world().resource::<UpdatePrefs>();
    assert_eq!(
        prefs,
        UpdatePrefs {
            check: false,
            install: false
        }
    );
    assert_eq!(requests(&mut app), [UpdateRequest::Prefs(prefs)]);

    let now = button(&mut app, &UpdateButton::CheckNow);
    click(&mut app, now);
    assert_eq!(requests(&mut app), [UpdateRequest::CheckNow]);
    app.update();
    assert!(texts(&mut app).iter().any(|t| t == "Checking…"));
}

#[test]
fn the_tables_menu_has_a_line_only_when_there_is_news() {
    let mut app = headless();
    let lines = app
        .world_mut()
        .run_system_once(|mut commands: Commands, fonts: Res<UiFonts>| {
            let empty = UpdateNotice::default();
            let news = UpdateNotice {
                shown: Some(ready()),
                ..UpdateNotice::default()
            };
            [
                menu_line(&mut commands, &fonts, Lang::De, None, 12.0),
                menu_line(&mut commands, &fonts, Lang::De, Some(&empty), 12.0),
                menu_line(&mut commands, &fonts, Lang::De, Some(&news), 12.0),
            ]
        })
        .expect("the lines build");
    assert!(lines[0].is_none() && lines[1].is_none());
    let line = lines[2].expect("a line for news");
    app.update();
    let text = app.world().get::<Text>(line).expect("a text");
    assert_eq!(
        text.0,
        "Update 0.1.0-beta.3 bereit – wird beim Beenden installiert"
    );
    assert_eq!(
        app.world().get::<UpdateButton>(line),
        Some(&UpdateButton::Open(
            "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into()
        )),
        "pressing it opens the release notes"
    );
}

#[test]
fn the_choice_defaults_to_on_and_reads_back() {
    let off: UpdatePrefs = serde_json::from_str(r#"{"check":false}"#).expect("parses");
    assert_eq!(
        off,
        UpdatePrefs {
            check: false,
            install: true
        },
        "a missing switch is on"
    );
    let text = serde_json::to_string(&off).expect("writes");
    assert_eq!(
        serde_json::from_str::<UpdatePrefs>(&text).expect("reads"),
        off
    );
}

/// The pages `open` was asked for. Systems run on the executor's threads,
/// so a lock rather than a thread-local; one test clicks a link.
pub(super) fn opened() -> std::sync::MutexGuard<'static, Vec<String>> {
    static OPENED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    OPENED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn read_only_place() -> UpdatePlace {
    UpdatePlace {
        read_only: Some((
            "/Applications".into(),
            "Permission denied (os error 13)".into(),
        )),
        moves: vec![MoveTo::Home, MoveTo::System],
        ..UpdatePlace::default()
    }
}

/// A read-only folder: the notice names the folder and the error, and
/// offers both moves, which reach the updater as requests.
#[test]
fn a_read_only_folder_is_named_and_the_move_is_offered() {
    let mut app = headless();
    app.world_mut().insert_resource(read_only_place());
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Available {
        version: "0.1.0-beta.6".into(),
        page: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.6".into(),
        why: Why::Folder,
    });
    app.update();
    let said = texts(&mut app);
    assert!(
        said.iter().any(|t| t
            == "Baylee installs no updates here: it may not write to /Applications \
                (Permission denied (os error 13)). Move it to your Applications folder and it can."),
        "{said:?}"
    );
    assert!(said.iter().any(|t| t == "Move to my Applications folder"));
    assert!(
        said.iter()
            .any(|t| t == "Move to Applications for all users")
    );
    let home = button(&mut app, &UpdateButton::Move(MoveTo::Home));
    click(&mut app, home);
    assert_eq!(requests(&mut app), [UpdateRequest::Move(MoveTo::Home)]);
}

/// An update that is only a link for another reason offers no move.
#[test]
fn no_move_is_offered_when_the_place_is_not_the_reason() {
    let mut app = headless();
    app.world_mut().insert_resource(read_only_place());
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Available {
        version: "0.1.0-beta.6".into(),
        page: "https://example.org/".into(),
        why: Why::NotVerified,
    });
    app.update();
    assert!(
        !texts(&mut app)
            .iter()
            .any(|t| t == "Move to my Applications folder")
    );
}

/// The first start after a move asks about the old copy, with no update
/// to announce, and the answers reach the updater.
#[test]
fn after_a_move_the_old_copy_is_offered_to_the_trash() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdatePlace>().moved = Some((
        "/Users/p/Applications/Baylee.app".into(),
        "/Users/p/Downloads/b/Baylee.app".into(),
    ));
    app.update();
    assert_eq!(toasts(&mut app), 1);
    let said = texts(&mut app);
    assert!(
        said.iter()
            .any(|t| t == "Baylee now runs from /Users/p/Applications/Baylee.app.")
    );
    assert!(
        said.iter()
            .any(|t| t == "The old copy is still at /Users/p/Downloads/b/Baylee.app.")
    );
    let trash = button(&mut app, &UpdateButton::TrashOld);
    click(&mut app, trash);
    assert_eq!(requests(&mut app), [UpdateRequest::TrashOld]);
    let keep = button(&mut app, &UpdateButton::KeepOld);
    click(&mut app, keep);
    assert_eq!(requests(&mut app), [UpdateRequest::KeepOld]);
}

/// The settings screen says where the app lies and offers the move, and
/// follows a failure without being reopened.
#[test]
fn the_settings_say_where_the_app_lies() {
    let mut app = headless();
    let row = app.world_mut().spawn((PlaceRow, Node::default())).id();
    app.update();
    assert!(
        app.world()
            .entity(row)
            .get::<Children>()
            .is_none_or(|children| children.iter().next().is_none()),
        "nothing to say, nothing said"
    );
    app.world_mut().insert_resource(UpdatePlace {
        translocated: true,
        moves: vec![MoveTo::Home],
        ..UpdatePlace::default()
    });
    app.update();
    let said = texts(&mut app);
    assert!(
        said.iter()
            .any(|t| t.starts_with("macOS runs Baylee from a read-only copy")),
        "{said:?}"
    );
    assert!(said.iter().any(|t| t == "Move to my Applications folder"));
    app.world_mut().resource_mut::<UpdatePlace>().failed =
        Some("Baylee could not be moved: full".into());
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "Baylee could not be moved: full")
    );
}

#[test]
fn only_a_web_page_is_opened() {
    assert!(openable(
        "https://github.com/AceVik/baylee/releases/tag/v0.1.0"
    ));
    assert!(openable("http://127.0.0.1:8080/release"));
    assert!(!openable("http://github.com/AceVik/baylee/releases"));
    assert!(!openable("http://127.0.0.1.evil.example/"));
    assert!(!openable("file:///etc/passwd"));
    assert!(!openable("javascript:alert(1)"));
}
