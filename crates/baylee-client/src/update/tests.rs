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
fn a_staged_update_is_announced_in_the_lobby_and_not_over_a_table() {
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
    assert_eq!(toasts(&mut app), 0, "the table's menu says it there");

    phase(&mut app, DuelPhase::Closed);
    app.update();
    assert_eq!(toasts(&mut app), 1, "back in the lobby, it is back");
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
fn hide_puts_the_notice_away_and_release_notes_open_the_page() {
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

    let hide = button(&mut app, &UpdateButton::Hide);
    click(&mut app, hide);
    app.update();
    assert!(app.world().resource::<UpdateNotice>().hidden);
    assert_eq!(toasts(&mut app), 0);
}

#[test]
fn the_switches_flip_this_devices_choice_and_tell_the_thread() {
    let mut app = headless();
    app.world_mut()
        .run_system_once(|mut commands: Commands, fonts: Res<UiFonts>| {
            controls(&mut commands, &fonts, Metrics::of(1280.0), Lang::En);
        })
        .expect("the controls build");
    app.update();
    assert_eq!(
        *app.world().resource::<UpdatePrefs>(),
        UpdatePrefs::default(),
        "both on by default"
    );
    assert_eq!(
        texts(&mut app).iter().filter(|t| *t == "on").count(),
        2,
        "{:?}",
        texts(&mut app)
    );

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
    assert_eq!(texts(&mut app).iter().filter(|t| *t == "off").count(), 1);

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
