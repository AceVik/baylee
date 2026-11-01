//! The updater's face in a headless app: the notice appears where it
//! should, its buttons do what they say, and the switches reach the thread.

use super::card::*;
use super::settings::*;
use super::*;
use crate::hud::{UiFonts, tf};
use crate::lobby::Metrics;
use crate::settings::ClientSettings;
use crate::{DuelPhase, lobby};
use bevy::ecs::system::RunSystemOnce;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerClick;
use bevy::ui::{percent, px};

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
        .add_message::<PointerClick>()
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
    let event = crate::test_pointer::Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut().write_message(crate::test_pointer::aim(
        PointerId::Mouse,
        location,
        event,
        entity,
    ));
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
            .any(|t| t == "Update 0.1.0-beta.3 is ready"),
        "{:?}",
        texts(&mut app)
    );

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
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Updated {
        version: "0.1.0-beta.3".into(),
        page: Some("https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.3".into()),
    });
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

    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(ready());
    app.update();
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

fn fetching(stage: Fetch) -> Shown {
    Shown::Fetching {
        version: "0.1.0-beta.9".into(),
        page: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.9".into(),
        stage,
    }
}

fn reading(received: u64) -> Progress {
    Progress {
        received,
        total: 150_000_000,
        rate: Some(3_200_000),
        left: Some((150_000_000 - received) / 3_200_000),
    }
}

/// The one entity marked `M`, and the tick it was last written at.
fn marked<M: Component>(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<M>>()
        .single(world)
        .unwrap_or_else(|_| panic!("one {}", std::any::type_name::<M>()))
}

fn text_of(app: &App, entity: Entity) -> String {
    app.world().get::<Text>(entity).expect("a text").0.clone()
}

/// A download draws its bar and its two numbers, and a reading moves them
/// in place: the card is not rebuilt for it.
#[test]
fn a_download_fills_its_bar_and_says_how_far_in_place() {
    let mut app = headless();
    {
        let mut notice = app.world_mut().resource_mut::<UpdateNotice>();
        notice.shown = Some(fetching(Fetch::Downloading));
        notice.progress = Some(reading(45_000_000));
    }
    app.update();
    assert_eq!(toasts(&mut app), 1);
    let card = marked::<UpdateToast>(&mut app);
    let said = texts(&mut app);
    for wanted in ["Update 0.1.0-beta.9", "Downloading…"] {
        assert!(said.iter().any(|t| t == wanted), "{wanted}: {said:?}");
    }
    let fill = marked::<ProgressFill>(&mut app);
    let share = marked::<ProgressPercent>(&mut app);
    let line = marked::<ProgressLine>(&mut app);
    assert_eq!(app.world().get::<Node>(fill).unwrap().width, percent(30.0));
    assert_eq!(text_of(&app, share), "30%");
    assert_eq!(
        text_of(&app, line),
        "45 of 150 MB · 3.2 MB/s · about 30 s left"
    );

    app.world_mut().resource_mut::<UpdateNotice>().progress = Some(reading(120_000_000));
    app.update();
    assert_eq!(marked::<UpdateToast>(&mut app), card, "moved in place");
    assert_eq!(app.world().get::<Node>(fill).unwrap().width, percent(80.0));
    assert_eq!(text_of(&app, share), "80%");
    assert_eq!(
        text_of(&app, line),
        "120 of 150 MB · 3.2 MB/s · about 10 s left"
    );
    assert!(
        app.world().get::<UpdateButton>(card).is_none()
            && button(&mut app, &UpdateButton::Hide) != card,
        "the card hides by its close mark"
    );

    // Verifying: the whole bar, and what was downloaded under it.
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(fetching(Fetch::Verifying));
    app.update();
    let fill = marked::<ProgressFill>(&mut app);
    assert_eq!(app.world().get::<Node>(fill).unwrap().width, percent(100.0));
    let said = texts(&mut app);
    for wanted in ["Verifying…", "150 MB downloaded"] {
        assert!(said.iter().any(|t| t == wanted), "{wanted}: {said:?}");
    }
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(fetching(Fetch::Unpacking));
    app.update();
    let said = texts(&mut app);
    for wanted in ["Unpacking…", "Signature verified"] {
        assert!(said.iter().any(|t| t == wanted), "{wanted}: {said:?}");
    }
}

/// At rest the progress writer writes nothing: a frame without a new
/// reading leaves the bar's node and both numbers unchanged, and a reading
/// that moves nothing drawn writes nothing either. Proved by a reading that
/// does move the bar.
#[test]
fn a_download_at_rest_writes_nothing() {
    let mut app = headless();
    {
        let mut notice = app.world_mut().resource_mut::<UpdateNotice>();
        notice.shown = Some(fetching(Fetch::Downloading));
        notice.progress = Some(reading(45_000_000));
    }
    app.update();
    app.update();
    let fill = marked::<ProgressFill>(&mut app);
    let share = marked::<ProgressPercent>(&mut app);
    let ticks = |app: &App| {
        let world = app.world();
        (
            world
                .entity(fill)
                .get_change_ticks::<Node>()
                .unwrap()
                .changed,
            world
                .entity(share)
                .get_change_ticks::<Text>()
                .unwrap()
                .changed,
        )
    };
    let before = ticks(&app);
    app.update();
    app.update();
    assert_eq!(ticks(&app), before, "nothing written at rest");
    // A reading 10 kB on: the same tenth of a percent, the same words.
    app.world_mut().resource_mut::<UpdateNotice>().progress = Some(reading(45_010_000));
    app.update();
    assert_eq!(ticks(&app), before, "a reading that moves nothing drawn");
    app.world_mut().resource_mut::<UpdateNotice>().progress = Some(reading(60_000_000));
    app.update();
    let after = ticks(&app);
    assert!(
        after.0 != before.0 && after.1 != before.1,
        "a real reading writes"
    );
}

/// A download, a failure and a link are never drawn over a table: the
/// menu's line says them there, and follows the download's share.
#[test]
fn a_download_stays_off_the_table_and_the_menu_follows_it() {
    let mut app = headless();
    phase(&mut app, DuelPhase::Playing);
    table(&mut app, None);
    {
        let mut notice = app.world_mut().resource_mut::<UpdateNotice>();
        notice.shown = Some(fetching(Fetch::Downloading));
        notice.progress = Some(reading(45_000_000));
    }
    app.update();
    assert_eq!(toasts(&mut app), 0, "no card at a table while downloading");
    let line = app
        .world_mut()
        .run_system_once(
            |mut commands: Commands, fonts: Res<UiFonts>, notice: Res<UpdateNotice>| {
                menu_line(&mut commands, &fonts, Lang::En, Some(&notice), 12.0)
            },
        )
        .expect("the line builds")
        .expect("a line while downloading");
    app.update();
    assert_eq!(text_of(&app, line), "Downloading update 0.1.0-beta.9 · 30%");
    app.world_mut().resource_mut::<UpdateNotice>().progress = Some(reading(75_000_000));
    app.update();
    assert_eq!(text_of(&app, line), "Downloading update 0.1.0-beta.9 · 50%");
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Failed {
        version: "0.1.0-beta.9".into(),
        page: "https://example.invalid/".into(),
        reason: "connection reset".into(),
    });
    app.update();
    assert_eq!(toasts(&mut app), 0, "no failure at a table either");
}

/// A failed download says why and that the next check tries again, and
/// "Try again" asks for a check now and says it is checking.
#[test]
fn a_failed_download_says_why_and_tries_again_when_asked() {
    let mut app = headless();
    app.world_mut().resource_mut::<UpdateNotice>().shown = Some(Shown::Failed {
        version: "0.1.0-beta.9".into(),
        page: "https://example.invalid/".into(),
        reason: "connection reset".into(),
    });
    app.update();
    let said = texts(&mut app);
    assert!(
        said.iter().any(|t| t
            == "The download stopped (connection reset). Baylee tries again at the next check."),
        "{said:?}"
    );
    let again = button(&mut app, &UpdateButton::TryAgain);
    click(&mut app, again);
    assert_eq!(requests(&mut app), [UpdateRequest::CheckNow]);
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == Phrase::UpdateChecking.text(Lang::En)),
        "the card says the retry is under way"
    );
}

/// The settings' status line says how far a download is, whoever started
/// it.
#[test]
fn the_settings_status_follows_a_download() {
    let mut app = headless();
    let status = app.world_mut().spawn((Text::new(""), Readout::Status)).id();
    {
        let mut notice = app.world_mut().resource_mut::<UpdateNotice>();
        notice.shown = Some(fetching(Fetch::Downloading));
        notice.progress = Some(reading(45_000_000));
        notice.asked = Some(Asked::Checking);
    }
    app.update();
    assert_eq!(
        text_of(&app, status),
        "Downloading update 0.1.0-beta.9 · 45 of 150 MB · 3.2 MB/s · about 30 s left"
    );
}

/// German, the decimal comma and the German time left.
#[test]
fn a_german_download_reads_in_german() {
    let mut app = headless();
    app.world_mut().insert_resource(ClientSettings {
        lang: "de".into(),
        ..default()
    });
    {
        let mut notice = app.world_mut().resource_mut::<UpdateNotice>();
        notice.shown = Some(fetching(Fetch::Downloading));
        notice.progress = Some(reading(45_000_000));
    }
    app.update();
    let line = marked::<ProgressLine>(&mut app);
    let share = marked::<ProgressPercent>(&mut app);
    assert_eq!(
        text_of(&app, line),
        "45 von 150 MB · 3,2 MB/s · noch etwa 30 s"
    );
    assert_eq!(text_of(&app, share), "30\u{a0}%");
    assert!(texts(&mut app).iter().any(|t| t == "Wird heruntergeladen…"));
}

/// Real Bevy text and flex layout, with shipped fonts; no renderer, native
/// updater, gateway or injected `ComputedNode` measurements.
struct NoticeLayout {
    app: App,
    window: Entity,
    camera: Entity,
    canvas: Entity,
    footer: Option<Entity>,
}

impl NoticeLayout {
    #[allow(clippy::too_many_lines)] // complete real Bevy layout fixture, including the footer
    fn new(size: UVec2, footer_height: Option<f32>, lang: Lang, scale: f32) -> Self {
        use bevy::camera::{ComputedCameraValues, RenderTargetInfo};
        let (mut app, fonts) = crate::face::tests::layout_app();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<DuelPhase>()
            .add_message::<PointerClick>()
            .insert_resource(fonts.clone())
            .insert_resource(ClientSettings {
                lang: lang.code().into(),
                ..default()
            })
            .insert_resource(UiScale(scale))
            .add_plugins(UpdatePlugin);
        let window = app
            .world_mut()
            .spawn(Window {
                resolution: bevy::window::WindowResolution::new(size.x, size.y)
                    .with_scale_factor_override(1.0),
                ..default()
            })
            .id();
        // Camera target metadata as in bevy_ui's own headless layout tests.
        let camera = app
            .world_mut()
            .spawn((
                Camera2d,
                bevy::ui::IsDefaultUiCamera,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size,
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
            ))
            .id();
        let canvas = app
            .world_mut()
            .spawn(Node {
                width: px(size.as_vec2().x / scale),
                height: px(size.as_vec2().y / scale),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        let footer = footer_height.map(|height| {
            app.world_mut().entity_mut(canvas).insert(lobby::LobbyRoot);
            let page = app
                .world_mut()
                .spawn((
                    crate::shellkit::role::Role::Scroll,
                    Node {
                        width: percent(100),
                        flex_grow: 1.0,
                        min_height: px(0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .id();
            let foot = app
                .world_mut()
                .spawn(Node {
                    width: percent(100),
                    height: px(height / scale),
                    flex_shrink: 0.0,
                    ..default()
                })
                .id();
            let words = app
                .world_mut()
                .spawn((
                    Text::new(format!(
                        "{} · {} · Source (AGPL-3.0)",
                        Phrase::ColophonNotice.text(lang),
                        Phrase::ColophonScryfall.text(lang)
                    )),
                    tf(&fonts, 11.5),
                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                ))
                .id();
            app.world_mut().entity_mut(foot).add_child(words);
            app.world_mut()
                .entity_mut(canvas)
                .add_children(&[page, foot]);
            foot
        });
        if footer.is_none() {
            app.world_mut()
                .resource_mut::<NextState<DuelPhase>>()
                .set(DuelPhase::Playing);
        }
        Self {
            app,
            window,
            camera,
            canvas,
            footer,
        }
    }

    fn show(&mut self, shown: Option<Shown>, place: UpdatePlace) -> Entity {
        self.app.world_mut().resource_mut::<UpdateNotice>().shown = shown;
        self.app.world_mut().insert_resource(place);
        self.app.update();
        let world = self.app.world_mut();
        let toast = world
            .query_filtered::<Entity, With<UpdateToast>>()
            .single(world)
            .unwrap();
        self.app
            .world_mut()
            .entity_mut(self.canvas)
            .add_child(toast);
        self.settle();
        toast
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.app.update();
        }
    }

    fn wheel(&mut self, entity: Entity, y: f32) {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::window::WindowRef;
        let location = Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(self.window)
                    .normalize(Some(self.window))
                    .unwrap(),
            ),
            position: Vec2::ZERO,
        };
        self.app.world_mut().write_message(crate::test_pointer::aim(
            PointerId::Mouse,
            location,
            crate::test_pointer::Scroll {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y,
                hit: bevy::picking::backend::HitData::new(self.camera, 0.0, None, None),
                phase: bevy::input::touch::TouchPhase::Moved,
            },
            entity,
        ));
        self.settle();
    }

    fn rect(&self, entity: Entity) -> Rect {
        let world = self.app.world();
        let node = world.get::<ComputedNode>(entity).expect("real UI layout");
        let at = world
            .get::<bevy::ui::UiGlobalTransform>(entity)
            .expect("real UI placement");
        let scale = node.inverse_scale_factor() * world.resource::<UiScale>().0;
        Rect::from_center_size(at.translation * scale, node.size() * scale)
    }

    fn assert_buttons_clear(&mut self, toast: Entity, top: f32, bottom: f32) {
        let frame = self.rect(toast);
        let size = self.app.world().get::<Window>(self.window).unwrap();
        assert!(
            frame.width() > 0.0 && frame.height() > 0.0,
            "nonvacuous layout: {frame:?}"
        );
        assert!(
            frame.min.x >= 0.0 && frame.max.x <= size.width() + 0.5,
            "{frame:?}"
        );
        assert!(
            frame.min.y >= top - 0.5 && frame.max.y <= bottom + 0.5,
            "{frame:?}"
        );
        let world = self.app.world_mut();
        let buttons: Vec<_> = world
            .query_filtered::<Entity, With<UpdateButton>>()
            .iter(world)
            .collect();
        assert!(!buttons.is_empty());
        for button in buttons {
            let rect = self.rect(button);
            assert!(rect.height() >= 37.5, "no squeezed action: {rect:?}");
            assert!(
                rect.min.x >= frame.min.x && rect.max.x <= frame.max.x + 0.5,
                "{rect:?}"
            );
            assert!(
                rect.min.y >= frame.min.y && rect.max.y <= frame.max.y + 0.5,
                "{rect:?}"
            );
        }
    }
}

#[test]
fn the_ready_buttons_clear_the_measured_footer_after_resize_without_rebuilding() {
    for lang in [Lang::En, Lang::De] {
        let mut fixture = NoticeLayout::new(UVec2::new(960, 700), Some(56.0), lang, 1.0);
        let toast = fixture.show(Some(ready()), UpdatePlace::default());
        let footer = fixture.footer.unwrap();
        let bottom = fixture.rect(footer).min.y;
        assert!((bottom - 644.0).abs() < 0.5, "the reported 960x700 repro");
        fixture.assert_buttons_clear(toast, 16.0, bottom - 16.0);

        fixture
            .app
            .world_mut()
            .get_mut::<Window>(fixture.window)
            .unwrap()
            .resolution
            .set(640.0, 360.0);
        fixture
            .app
            .world_mut()
            .get_mut::<Camera>(fixture.camera)
            .unwrap()
            .computed
            .target_info
            .as_mut()
            .unwrap()
            .physical_size = UVec2::new(640, 360);
        {
            let mut node = fixture
                .app
                .world_mut()
                .get_mut::<Node>(fixture.canvas)
                .unwrap();
            node.width = px(640);
            node.height = px(360);
        }
        fixture
            .app
            .world_mut()
            .get_mut::<Node>(footer)
            .unwrap()
            .height = px(80);
        fixture.settle();
        assert!(
            fixture.app.world().get::<UpdateToast>(toast).is_some(),
            "resize retained the controls"
        );
        let bottom = fixture.rect(footer).min.y;
        fixture.assert_buttons_clear(toast, 16.0, bottom - 16.0);
    }
}

#[test]
fn every_lobby_notice_keeps_its_actions_visible_when_long_text_scrolls() {
    for lang in [Lang::En, Lang::De] {
        for state in 0..4 {
            let mut fixture = NoticeLayout::new(UVec2::new(640, 360), Some(80.0), lang, 1.0);
            let mut place = read_only_place();
            place.failed = Some("The operation could not finish. ".repeat(80));
            let shown = match state {
                0 => Some(ready()),
                1 => Some(Shown::Available {
                    version: "9.9.9".into(),
                    page: "https://example.invalid/".into(),
                    why: Why::Folder,
                }),
                2 => Some(Shown::Updated {
                    version: "9.9.9".into(),
                    page: Some("https://example.invalid/".into()),
                }),
                _ => {
                    place.moved = Some((
                        "/Applications/Baylee.app".into(),
                        "/Downloads/Baylee.app".into(),
                    ));
                    None
                }
            };
            let toast = fixture.show(shown, place);
            let bottom = fixture.rect(fixture.footer.unwrap()).min.y;
            fixture.assert_buttons_clear(toast, 16.0, bottom - 16.0);
            let world = fixture.app.world_mut();
            let body = world
                .query_filtered::<Entity, (With<ToastScroll>, Without<ToastActions>)>()
                .single(world)
                .unwrap();
            let node = world.get::<ComputedNode>(body).unwrap();
            // At least the ready/move/old-copy states carry the long failure.
            if state != 2 {
                assert!(node.content_size().y > node.size().y + 1.0);
                let restart_or_hide = button(
                    &mut fixture.app,
                    if state == 0 {
                        &UpdateButton::RestartNow
                    } else {
                        &UpdateButton::Hide
                    },
                );
                let before = fixture.rect(restart_or_hide);
                fixture.wheel(body, -4.0);
                assert!(fixture.app.world().get::<ScrollPosition>(body).unwrap().y > 0.0);
                assert_eq!(
                    fixture.rect(restart_or_hide),
                    before,
                    "reading text does not move actions"
                );
            }
        }
    }
}

#[test]
fn the_table_offer_fits_above_the_hand_in_small_and_scaled_desktop_windows() {
    for (size, scale) in [
        (UVec2::new(640, 360), 1.0),
        (UVec2::new(320, 480), 1.0),
        (UVec2::new(2560, 1440), 1.25),
    ] {
        let mut fixture = NoticeLayout::new(size, None, Lang::En, scale);
        let toast = fixture.show(Some(ready()), UpdatePlace::default());
        let bottom = size.as_vec2().y - (crate::hud::HAND_ZONE_H + TOAST_GAP) * scale;
        let top = crate::hud::TOP_CLEAR * scale;
        fixture.assert_buttons_clear(toast, top, bottom);
        let rect = fixture.rect(toast);
        assert!(
            (rect.center().x - size.as_vec2().x * 0.5).abs() < 0.5,
            "centred: {rect:?}"
        );
    }
}

#[test]
fn a_short_table_can_scroll_wrapped_german_actions_without_covering_the_ledge() {
    let mut fixture = NoticeLayout::new(UVec2::new(640, 360), None, Lang::De, 1.0);
    let toast = fixture.show(Some(ready()), UpdatePlace::default());
    let world = fixture.app.world_mut();
    let actions = world
        .query_filtered::<Entity, With<ToastActions>>()
        .single(world)
        .unwrap();
    let last = button(&mut fixture.app, &UpdateButton::Later);
    fixture.wheel(actions, -100.0);
    let viewport = fixture.rect(actions);
    let last_rect = fixture.rect(last);
    assert!(
        last_rect.min.y >= viewport.min.y - 0.5 && last_rect.max.y <= viewport.max.y + 0.5,
        "the complete last action is reachable: {last_rect:?} in {viewport:?}"
    );
    assert!(fixture.rect(toast).max.y <= 360.0 - crate::hud::HAND_ZONE_H - TOAST_GAP + 0.5);
    fixture.wheel(actions, 100.0);
    let first = button(&mut fixture.app, &UpdateButton::RestartNow);
    let button = fixture.rect(first);
    assert!(
        button.min.y >= viewport.min.y - 0.5 && button.max.y <= viewport.max.y + 0.5,
        "the complete first action is reachable: {button:?} in {viewport:?}"
    );
}

#[test]
fn a_signed_in_german_notice_keeps_its_explanation_above_wrapped_actions() {
    let mut fixture = NoticeLayout::new(UVec2::new(960, 700), None, Lang::De, 1.0);
    fixture
        .app
        .world_mut()
        .resource_mut::<NextState<DuelPhase>>()
        .set(DuelPhase::Closed);
    let toast = fixture.show(Some(ready()), UpdatePlace::default());
    fixture.settle();
    fixture.assert_buttons_clear(toast, 16.0, 684.0);
    let world = fixture.app.world_mut();
    let body = world
        .query_filtered::<Entity, (With<ToastScroll>, Without<ToastActions>)>()
        .single(world)
        .unwrap();
    let children = world.get::<Children>(body).unwrap().to_vec();
    let frame = fixture.rect(body);
    assert!(
        frame.height() >= 40.0,
        "the explanation must not collapse: {frame:?}"
    );
    assert!(children.len() >= 2, "headline and restart explanation");
    for child in children {
        let text = fixture.rect(child);
        assert!(
            text.min.y >= frame.min.y - 0.5 && text.max.y <= frame.max.y + 0.5,
            "short explanation fully visible: {text:?} in {frame:?}"
        );
    }
}

/// The download's card in real layout: it fits the window at a desktop
/// size, a small window and a phone's frame, in both languages; its bar
/// lies inside it; and at a desktop size the download's line takes one row.
#[test]
fn the_download_card_fits_desktop_small_and_phone_frames() {
    for lang in [Lang::En, Lang::De] {
        for size in [
            UVec2::new(1280, 800),
            UVec2::new(640, 360),
            UVec2::new(844, 390),
        ] {
            let mut fixture = NoticeLayout::new(size, Some(56.0), lang, 1.0);
            fixture
                .app
                .world_mut()
                .resource_mut::<UpdateNotice>()
                .progress = Some(reading(45_000_000));
            let toast = fixture.show(Some(fetching(Fetch::Downloading)), UpdatePlace::default());
            let bottom = fixture.rect(fixture.footer.unwrap()).min.y;
            fixture.assert_buttons_clear(toast, 16.0, bottom - 16.0);
            let frame = fixture.rect(toast);
            let world = fixture.app.world_mut();
            let track = world
                .query_filtered::<&ChildOf, With<ProgressFill>>()
                .single(world)
                .unwrap()
                .parent();
            let line = world
                .query_filtered::<Entity, With<ProgressLine>>()
                .single(world)
                .unwrap();
            let share = world
                .query_filtered::<Entity, With<ProgressPercent>>()
                .single(world)
                .unwrap();
            let track = fixture.rect(track);
            assert!(
                track.width() > 100.0 && track.height() >= 3.0,
                "a bar: {track:?}"
            );
            assert!(
                track.min.x >= frame.min.x && track.max.x <= frame.max.x,
                "{track:?} in {frame:?}"
            );
            // The title names the version on one row at every size: a text
            // in a row shrank to one character's width once.
            let world = fixture.app.world_mut();
            let body = world
                .query_filtered::<Entity, (With<ToastScroll>, Without<ToastActions>)>()
                .single(world)
                .unwrap();
            let title = world.get::<Children>(body).unwrap()[0];
            let stage = world
                .get::<Children>(world.get::<ChildOf>(share).unwrap().parent())
                .unwrap()[0];
            let one = fixture.rect(share).height();
            for (what, rect) in [
                ("title", fixture.rect(title)),
                ("stage", fixture.rect(stage)),
            ] {
                assert!(
                    rect.height() < one * 2.0 && rect.width() > 100.0,
                    "{lang:?} {size}: the {what} on one row: {rect:?}"
                );
            }
            if size.x >= 1280 {
                let under = fixture.rect(line);
                assert!(
                    under.height() < one * 1.6,
                    "{lang:?}: the line on one row: {under:?} against {one}"
                );
            }
        }
    }
}

/// The ready card's two actions stand on one row at a desktop size, in
/// both languages, the primary first.
#[test]
fn the_ready_card_s_actions_share_a_row_on_a_desktop() {
    for lang in [Lang::En, Lang::De] {
        let mut fixture = NoticeLayout::new(UVec2::new(1280, 800), Some(56.0), lang, 1.0);
        let toast = fixture.show(Some(ready()), UpdatePlace::default());
        let bottom = fixture.rect(fixture.footer.unwrap()).min.y;
        fixture.assert_buttons_clear(toast, 16.0, bottom - 16.0);
        let restart = button(&mut fixture.app, &UpdateButton::RestartNow);
        let later = button(&mut fixture.app, &UpdateButton::Later);
        let (restart, later) = (fixture.rect(restart), fixture.rect(later));
        assert!(
            (restart.center().y - later.center().y).abs() < 1.0,
            "{lang:?}: {restart:?} {later:?}"
        );
        assert!(restart.max.x <= later.min.x);
    }
}

/// The front door's page stands in the host its scroll bar shares with it
/// (`lobby::scrollbars::attach`), not in the lobby's root itself: the card
/// still stands above the colophon (live check of 22.10.2026, where it
/// covered the colophon's first words).
#[test]
fn the_card_clears_the_colophon_of_a_page_with_a_scroll_bar() {
    use crate::shellkit::role::Role;
    let mut fixture = NoticeLayout::new(UVec2::new(1280, 800), Some(56.0), Lang::En, 1.0);
    let world = fixture.app.world_mut();
    let page = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .find_map(|(entity, role)| (*role == Role::Scroll).then_some(entity))
        .expect("the page");
    let host = world
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_basis: px(0),
            min_height: px(0),
            ..default()
        })
        .id();
    world.entity_mut(fixture.canvas).insert_children(0, &[host]);
    world.entity_mut(host).add_child(page);
    let toast = fixture.show(Some(fetching(Fetch::Downloading)), UpdatePlace::default());
    let colophon = fixture.rect(fixture.footer.unwrap()).min.y;
    assert!((colophon - 744.0).abs() < 0.5, "the fixture's colophon");
    let card = fixture.rect(toast);
    assert!(
        card.max.y <= colophon - 16.0 + 0.5,
        "above the colophon: {card:?}"
    );
}
