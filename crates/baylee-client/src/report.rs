//! Reporting a problem (#309) and a crash (#310) to the gateway.
//!
//! Everything that decides — what a report carries, what the player allowed,
//! whose names are replaced, what the gateway's answer means — is in
//! `baylee_client_core::bugreport` and tested there. This is the shell round
//! it: the form over whatever screen is up (`form`), collecting what the
//! form may offer ([`gather`]), the request, the panic hook and the courier
//! that sends a crash report on the next start.
//!
//! One form for the lobby and the table, opened by `F8` (`Action::Report`),
//! by the button in the table's top-right corner (`corner`, over the end
//! screen too), by the row in the table's game menu and by the button beside
//! the music controls in the lobby. It sends to the gateway the lobby is
//! signed in to, with that session. Signed in nowhere, it sends straight to
//! the feedback service this build was given (`BAYLEE_FEEDBACK_PUBLIC_URL`)
//! or the settings name, under this device's random id, after a
//! confirmation listing what goes; knowing none, it says so and sends
//! nothing, to anyone.
//!
//! A report about a game this client hosted (against the house) may carry
//! that game's record, taken from the host when the form opens: a box
//! ticked for that one report, never remembered, confirmed before it goes.
//!
//! The text may name cards and players in brackets (window B): `#` offers
//! the seat's own view at a table and the compiled pool elsewhere, `@` the
//! other seats or the room's players ([`gather`], `refs`), and a finished
//! reference previews its card.

use std::sync::{Arc, Mutex};

use baylee_client_core::bugreport::{
    self, Build, Category, Consent, CrashConsent, CrashFile, CrashStep, Game, Gathered, Holding,
    Keyring, LocalRecord, ReportForm, Route, Settings, Status, System, Table, Via,
};
use baylee_client_core::i18n::Lang;
use baylee_client_core::lobby::{Screen, gateway_list};
use baylee_client_core::prefs::Action;
use bevy::prelude::*;

use crate::lobby::LobbyState;
use crate::settings::ClientSettings;

mod corner;
mod field;
mod form;
mod keys;
mod refs;
mod shot;
// The guided tour points at both (`crate::tour::table`).
pub(crate) use corner::ReportCorner;
#[cfg(test)]
pub(crate) use field::{DeskBox, DeskCaret, DeskSuggest, DeskSuggestMeta, DeskText, ReportLink};
/// The form's buttons: `devctl`'s `desk_controls` row, and the pointer's
/// shape over them (`shellkit::pointer`).
pub(crate) use form::DeskPress;
pub(crate) use form::DeskRoot;
#[cfg(test)]
pub(crate) use form::DeskScroll;
#[cfg(not(any(test, all(feature = "dev-control", not(target_arch = "wasm32")))))]
pub(crate) use form::{REPORT, REPORT_CONFIRM};
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
pub(crate) use form::{REPORT, REPORT_CONFIRM, ReportAnchor};
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
pub(crate) use refs::ReportPreview;
#[cfg(test)]
mod tests;

/// Where the panic hook leaves a crash, beside the settings.
const CRASH_FILE: &str = "crash-report.json";

/// The gateway the lobby is signed in to, for the panic hook to write down.
///
/// A static because the hook may read nothing from the world: the world may
/// be what broke. Written by [`remember_the_gateway`] whenever the session
/// changes.
static SIGNED_IN_AT: Mutex<Option<String>> = Mutex::new(None);

/// The report form and everything it holds between frames.
#[derive(Resource, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a different thing standing open or pending, as on `Duel`"
)]
pub struct ReportDesk {
    /// Whether the form is up.
    pub open: bool,
    /// Keys go to the form this frame even though it just closed, so the
    /// `Esc` that closed it does not also reach the screen under it.
    swallow: bool,
    /// The form's state.
    form: ReportForm,
    /// What the form may attach, collected when it opened.
    gathered: Gathered,
    /// Whether the screenshot is still being taken.
    shooting: bool,
    /// Frames the form has waited for the screenshot before drawing itself,
    /// so the picture is of the screen and not of the form.
    waited: u32,
    /// The crash found at start, while it waits for a session or an answer.
    crash: Option<CrashFile>,
    /// Whether the player is being asked about crash reports.
    asking: bool,
    /// Whether this start already tried to send the crash.
    crash_tried: bool,
    /// Asked for from elsewhere (a button, the game menu): open next frame.
    pub asked: bool,
    /// A clipboard read `Ctrl`/`Cmd`+`V` started, until it answers.
    paste: Option<bevy::clipboard::ClipboardRead>,
    /// How far the text box is scrolled, carried across the rebuild every
    /// keystroke makes of the form.
    box_scroll: f32,
    /// How far the form itself is scrolled, carried across the rebuild a
    /// tick makes: the boxes at its foot (the record's) are ticked scrolled
    /// down, and a form that jumped back to its top on each would hide the
    /// box just ticked.
    panel_scroll: f32,
    /// Whether the tree up is the form (not the confirmation or the crash
    /// question), whose scroll [`Self::panel_scroll`] keeps.
    drawn_form: bool,
    /// How often the tree has been drawn, for `devctl`'s `ui_rebuilds`.
    pub(crate) redraws: u64,
    /// Ctrl/Cmd+Enter in the text, or Enter on the confirmation: the
    /// form's Send or the confirmation's, on the next run of
    /// [`send_by_key`] (`KEYBOARD.md` W9).
    send_by_key: Option<form::DeskPress>,
    /// Whether the form was opened over a table: `#` offers that seat's
    /// view, never the pool.
    at_table: bool,
    /// The reading seat at that table.
    me: Option<baylee_core::ids::PlayerId>,
    /// The table's seats by number, as its roster names them, for the
    /// suggestions' "whose".
    seat_names: std::collections::BTreeMap<u8, String>,
    /// The reference the pointer is on (or a finger holds), for its preview.
    hover: Option<refs::ReportHover>,
    /// A suggestion was taken: the keyboard goes back to the text.
    refocus: bool,
    /// Whether Copy as text reached the clipboard, once pressed.
    copied: Option<bool>,
}

impl ReportDesk {
    /// Whether the keyboard belongs to the form (or the crash question)
    /// this frame. Every other key handler asks this first.
    #[must_use]
    pub fn holds_keyboard(&self) -> bool {
        self.open || self.asking || self.swallow
    }
}

/// What `/state.report` says of the sheet (window B): open or confirming,
/// the text, the suggestions standing (name and second column) and the row
/// the keys are on, and what the text names.
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
impl ReportDesk {
    pub(crate) fn state_json(&self, lang: Lang) -> serde_json::Value {
        let suggestions = self.form.suggestions(&self.gathered.refs);
        let rows: Vec<serde_json::Value> = suggestions
            .as_ref()
            .map(|list| {
                list.rows
                    .iter()
                    .map(|row| match *row {
                        bugreport::Suggestion::Card(i) => {
                            let card = &self.gathered.refs.cards[i];
                            serde_json::json!({
                                "name": card.name,
                                "meta": form::meta_of(card, self, lang),
                                "card": card.card.get(),
                            })
                        }
                        bugreport::Suggestion::Player(i) => {
                            serde_json::json!({ "name": self.gathered.refs.players[i].name })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let refs = bugreport::refs::refs_for(
            self.form.text.text(),
            &self.gathered.refs,
            &self.form.picked,
        )
        .and_then(|r| serde_json::to_value(r).ok());
        serde_json::json!({
            "open": self.open,
            "confirming": self.form.confirming,
            "at_table": self.at_table,
            "text": self.form.text.text(),
            "candidates": self.gathered.refs.cards.len(),
            "players": self.gathered.refs.players.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
            "suggestions": rows,
            "chosen": suggestions.as_ref().map(|list| self.form.chosen_in(list)),
            "refs": refs,
            "hover": self.hover.as_ref().map(|h| h.card.name.clone()),
        })
    }
}

/// What the tests in `report::tests` and elsewhere read and set of a desk
/// whose fields are this module's.
#[cfg(test)]
impl ReportDesk {
    /// Scrolls the text box, as `field::place_the_caret` does once the text
    /// is laid out — which a headless test never does.
    pub(crate) fn set_box_scroll(&mut self, y: f32) {
        self.box_scroll = y;
    }

    pub(crate) fn form(&self) -> &ReportForm {
        &self.form
    }
    pub(crate) fn form_mut(&mut self) -> &mut ReportForm {
        &mut self.form
    }
    pub(crate) fn gathered_mut(&mut self) -> &mut Gathered {
        &mut self.gathered
    }
    pub(crate) fn gathered_refs_len(&self) -> usize {
        self.gathered.refs.cards.len()
    }
    pub(crate) fn asking(&self) -> bool {
        self.asking
    }
    pub(crate) fn crash(&self) -> Option<&CrashFile> {
        self.crash.as_ref()
    }
    pub(crate) fn crash_tried(&self) -> bool {
        self.crash_tried
    }
    /// The clipboard's answer, as a desktop clipboard gives it: at once.
    pub(crate) fn clipboard_answers(&mut self, text: &str) {
        self.paste = Some(bevy::clipboard::ClipboardRead::Ready(Ok(text.to_string())));
    }
    /// A crash file found at start, as `find_a_crash` meets it.
    pub(crate) fn found(&mut self, consent: CrashConsent, text: Option<&str>) -> bool {
        meet_the_crash(self, consent, text)
    }
}

/// The gateway answering the form's report with `status` and `body`, for a
/// test standing in for the HTTP thread.
#[cfg(test)]
pub(crate) fn gateway_answers(world: &World, status: u16, body: &str) {
    if let Ok(mut slot) = world.resource::<Answers>().0.lock() {
        slot.push(Answer::Form(status, body.to_string()));
    }
}

/// The gateway's answers, from the HTTP thread.
#[derive(Resource, Default, Clone)]
struct Answers(Arc<Mutex<Vec<Answer>>>);

/// One answer: to the form's report, or to the crash report.
enum Answer {
    Form(u16, String),
    Crash(u16),
}

/// Installs the form, the courier and the start-up check.
pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<ReportDesk>() {
        return;
    }
    // A headless test builds the lobby without a settings file, and the
    // consent lives there: no settings, no form.
    let settled = resource_exists::<ClientSettings>;
    // And one without the input plugin: the form reads keys and the wheel.
    app.add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_message::<bevy::input::mouse::MouseWheel>()
        .add_message::<crate::shellkit::focus::Activated>()
        .add_observer(keys::pressed);
    app.init_resource::<ReportDesk>()
        .init_resource::<Answers>()
        .add_systems(Startup, find_a_crash.run_if(settled))
        // In `Update`, after every `PreUpdate` writer of keys (the
        // dev-control harness presses them there). The lobby's and the
        // table's handlers ask `holds_keyboard` whichever side of this they
        // run on: open, the form has the keys either way, and the frame it
        // closes on is swallowed by `swallow`.
        .add_systems(Update, keys.run_if(resource_exists::<ClientSettings>))
        .add_systems(
            Update,
            (
                corner::keep_the_corner,
                open_when_asked,
                remember_the_gateway,
                answers,
                activate_by_key,
                send_by_key,
                send_the_crash,
                refs::fill_the_pool,
                form::draw,
                form::retick,
                keys::scroll,
                field::blink,
            )
                .chain()
                .run_if(settled),
        )
        // The references' preview follows the pointer's messages, which the
        // picking plugin registers. Registered here instead, a headless
        // lobby's other readers of them woke up and moved its focus (the
        // terms sheet's Esc, `front_terms`), so here they are only read.
        .add_systems(
            Update,
            (refs::follow_the_links, refs::show_the_preview)
                .chain()
                .after(field::blink)
                .run_if(settled.and_then(refs::pointer_messages)),
        )
        .add_systems(
            Update,
            hold_the_focus
                .after(crate::shellkit::focus::FocusSystems)
                .after(form::draw)
                .run_if(settled),
        )
        .add_systems(
            PostUpdate,
            field::place_the_caret
                .after(bevy::ui::UiSystems::PostLayout)
                .run_if(settled),
        );
}

/// Opens the form on `F8` (or whatever the player bound it to), and hands
/// every key to it while it is up.
#[allow(clippy::too_many_arguments)] // a Bevy system: the keys and where focus stands
fn keys(
    codes: Res<ButtonInput<KeyCode>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut desk: ResMut<ReportDesk>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
    stops: Query<&crate::shellkit::focus::Stop>,
) {
    desk.swallow = false;
    keys::take_the_paste(&mut desk);
    if desk.asking {
        typed.clear();
        return;
    }
    if !desk.open {
        let opened = prefs.is_some_and(|prefs| {
            crate::keys::Binds::new(&codes, prefs.keymap()).just_pressed(Action::Report)
        });
        if opened {
            desk.asked = true;
        }
        typed.clear();
        return;
    }
    // The keyboard stands on another of the sheet's controls when Tab put
    // it there (the ring shows): then Enter and Space are that control's.
    let elsewhere = visible.is_some_and(|v| v.0)
        && focus
            .as_deref()
            .and_then(bevy::input_focus::InputFocus::get)
            .and_then(|f| stops.get(f).ok())
            .is_some_and(|s| {
                (s.table == form::REPORT.name && s.id != "text")
                    || (s.table == form::REPORT_CONFIRM.name && s.id != "send-now")
            });
    let closed = keys::typing(
        &mut desk,
        &codes,
        &mut typed,
        clipboard.as_deref_mut(),
        elsewhere,
    );
    // Natively the clipboard has answered already: land it this frame.
    keys::take_the_paste(&mut desk);
    if closed {
        desk.open = false;
        desk.swallow = true;
    }
}

/// Opens the form a button or the game menu asked for, gathering what it
/// may offer.
#[allow(clippy::too_many_arguments)] // one reader per thing a report can carry
fn open_when_asked(
    mut commands: Commands,
    mut desk: ResMut<ReportDesk>,
    duel: Option<ResMut<crate::Duel>>,
    host: Option<Res<crate::InstalledHost>>,
    settings: Res<ClientSettings>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    lobby: Option<Res<LobbyState>>,
) {
    let mut duel = duel;
    let from_menu = duel.as_ref().is_some_and(|d| d.report_asked);
    if let Some(duel) = duel.as_mut().filter(|_| from_menu) {
        duel.report_asked = false;
        duel.game_menu = false;
    }
    if !(desk.asked || from_menu) || desk.open {
        desk.asked = false;
        return;
    }
    desk.asked = false;
    desk.open = true;
    desk.form.status = Status::Editing;
    desk.form.preview = false;
    desk.form.opened();
    desk.panel_scroll = 0.0;
    desk.hover = None;
    desk.copied = None;
    desk.refocus = true;
    let at_table = phase.is_some_and(|phase| *phase.get() != crate::DuelPhase::Closed);
    let duel = duel.as_deref().filter(|_| at_table);
    desk.gathered = gather(
        duel,
        host.as_deref(),
        &settings,
        prefs.as_deref(),
        windows.single().ok(),
        adapter.as_deref(),
    );
    desk.at_table = at_table;
    desk.me = duel.and_then(|d| d.view.as_ref()).map(|v| v.seat);
    desk.seat_names = duel
        .and_then(|d| d.statics.as_ref())
        .map(|s| {
            s.seats
                .iter()
                .map(|seat| (seat.player.get(), seat.display_name.clone()))
                .collect()
        })
        .unwrap_or_default();
    desk.gathered.refs = references(duel, texts.as_deref(), lobby.as_deref());
    // Taken before the form is drawn over it, and whether or not the box is
    // ticked: it stays on this machine unless it is.
    desk.shooting = shot::take(&mut commands);
    desk.waited = 0;
}

/// What the report's text may name, read when the form opens: at a table
/// the seat's own view (`bugreport::refs::candidates`, the hidden-
/// information rule) and the other seats; elsewhere the players of the room
/// the player sits in, and no card until the first `#` loads the pool
/// (`refs::fill_the_pool`).
fn references(
    duel: Option<&crate::Duel>,
    texts: Option<&crate::cardtext::CardTexts>,
    lobby: Option<&LobbyState>,
) -> bugreport::Candidates {
    if let Some(duel) = duel {
        let Some(view) = duel.view.as_ref() else {
            return bugreport::Candidates::default();
        };
        let statics = duel.statics.as_ref();
        let lookup = |card: baylee_core::ids::CardIndex, face: u8| {
            texts.and_then(|texts| texts.face(card, face))
        };
        return bugreport::Candidates {
            cards: bugreport::refs::candidates(view, statics, &lookup),
            players: statics
                .map(|s| bugreport::refs::table_players(s, view.seat))
                .unwrap_or_default(),
        };
    }
    let players = lobby
        .and_then(|l| baylee_client_core::lobby::strips::my_waiting_table(&l.lobby))
        .map(|room| {
            room.seats
                .iter()
                .filter(|seat| !seat.you)
                .filter_map(|seat| {
                    seat.player.clone().map(|name| bugreport::PlayerRef {
                        name,
                        seat: u8::try_from(seat.seat).ok(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    bugreport::Candidates {
        cards: Vec::new(),
        players,
    }
}

/// Everything the form may offer, read now.
fn gather(
    duel: Option<&crate::Duel>,
    host: Option<&crate::InstalledHost>,
    settings: &ClientSettings,
    prefs: Option<&crate::prefs::Prefs>,
    window: Option<&Window>,
    adapter: Option<&bevy::render::renderer::RenderAdapterInfo>,
) -> Gathered {
    let networked = host.is_some_and(|h| h.0.link() != crate::host::LinkState::Local);
    let statics = duel.and_then(|d| d.statics.as_ref());
    let game = duel.and_then(|duel| {
        let view = duel.view.clone()?;
        Some(Game {
            table: Table {
                seat: view.seat.get(),
                seq: view.seq,
                when: format!("turn {}, {:?}", view.turn, view.step),
            },
            pending: duel.interaction.as_ref().map(|i| i.pending().clone()),
            holding: Holding {
                selected: duel
                    .interaction
                    .as_ref()
                    .map_or(0, |i| i.selected().count()),
                armed: duel.armed.as_ref().map(|armed| format!("{:?}", armed.deed)),
                mana_run: duel.mana_run.is_some(),
                outbox: duel.outbox().len(),
                last_error: duel
                    .last_error
                    .as_ref()
                    .map(|refusal| refusal.text(Lang::En)),
            },
            view,
        })
    });
    let log = duel.and_then(|duel| {
        let seat = duel.view.as_ref()?.seat;
        let none = |_: baylee_core::ids::CardIndex, _: u8| None;
        Some(bugreport::seat_log(&duel.log, statics, seat, &none))
    });
    Gathered {
        build: build(),
        game_id: statics
            .filter(|_| networked)
            .map(|statics| statics.game_id.clone()),
        system: Some(system(window, adapter, &settings.lang)),
        game,
        log,
        settings: Some(Settings {
            lang: settings.lang.clone(),
            preview_scale: settings.preview_scale,
            prefer_text_view: settings.prefer_text_view,
            zone_view: format!("{:?}", settings.zone_view),
            music: format!("{:?}", settings.music),
            saved_gateways: settings.gateways.len(),
            preferences: prefs.and_then(|p| serde_json::from_str(&p.all().to_json()).ok()),
        }),
        screenshot: None,
        // Packed here, once per opening, and not per keystroke: the form
        // rebuilds its body on every change it shows.
        local_record: host
            .and_then(|h| h.0.local_record())
            .and_then(LocalRecord::pack),
        refs: bugreport::Candidates::default(),
    }
}

/// Enter or Space on one of the sheet's controls the keyboard stands on
/// (the kit's walker says which): the same press a click is.
#[allow(clippy::too_many_arguments)] // a Bevy system: one reader per thing a press may change
fn activate_by_key(
    mut activated: MessageReader<crate::shellkit::focus::Activated>,
    presses: Query<&form::DeskPress>,
    mut desk: ResMut<ReportDesk>,
    mut settings: ResMut<ClientSettings>,
    holders: Holders,
    answers: Res<Answers>,
    lobby: Option<Res<LobbyState>>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    for event in activated.read() {
        let ours =
            event.stop.table == form::REPORT.name || event.stop.table == form::REPORT_CONFIRM.name;
        if !(event.by_key && ours) {
            continue;
        }
        if let Ok(&press) = presses.get(event.entity) {
            keys::act(
                press,
                &mut desk,
                &mut settings,
                &holders,
                &answers,
                lobby.as_deref(),
                clipboard.as_deref_mut(),
            );
        }
    }
}

/// Keeps the keyboard on the sheet: on the text when the form opens or a
/// suggestion was taken, and on one of the sheet's stops while it is up (a
/// click elsewhere on the screen under it does not take it away); on Send
/// now while the confirmation is up.
fn hold_the_focus(
    mut desk: ResMut<ReportDesk>,
    focus: Option<ResMut<bevy::input_focus::InputFocus>>,
    stops: Query<(Entity, &crate::shellkit::focus::Stop)>,
) {
    let Some(mut focus) = focus else {
        return;
    };
    if !desk.open {
        return;
    }
    let (table, home) = if desk.form.confirming {
        (form::REPORT_CONFIRM.name, "send-now")
    } else {
        (form::REPORT.name, "text")
    };
    let here = focus.get().and_then(|f| stops.get(f).ok()).map(|(_, s)| *s);
    let held = here.is_some_and(|s| s.table == table);
    if held && !desk.refocus {
        return;
    }
    let Some((entity, _)) = stops.iter().find(|(_, s)| s.table == table && s.id == home) else {
        // Not drawn yet (the form waits for its picture): ask again.
        return;
    };
    if focus.get() != Some(entity) {
        focus.set(entity, bevy::input_focus::FocusCause::Navigated);
    }
    if desk.refocus {
        desk.refocus = false;
    }
}

/// Which build this is.
fn build() -> Build {
    Build {
        version: baylee_build::short().to_string(),
        commit: Some(baylee_build::COMMIT.to_string()).filter(|c| !c.is_empty()),
    }
}

/// What this runs on.
fn system(
    window: Option<&Window>,
    adapter: Option<&bevy::render::renderer::RenderAdapterInfo>,
    lang: &str,
) -> System {
    System {
        platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        cpus: std::thread::available_parallelism().ok().map(usize::from),
        adapter: adapter.map(|a| a.name.clone()),
        backend: adapter.map(|a| format!("{:?}", a.backend)),
        window: window.map_or((0, 0), |w| {
            (
                w.resolution.width().round() as u32,
                w.resolution.height().round() as u32,
            )
        }),
        scale: window.map_or(1.0, |w| w.resolution.scale_factor()),
        lang: lang.to_string(),
    }
}

/// The feedback service this build was given, if any.
const BUILT_FEEDBACK_URL: Option<&str> = option_env!("BAYLEE_FEEDBACK_PUBLIC_URL");

/// Where a report goes now: the gateway the lobby is signed in to, else
/// the feedback service the settings or the build name, else nowhere.
pub(crate) fn route(lobby: Option<&LobbyState>, settings: &ClientSettings) -> Route {
    let service = bugreport::feedback_service(settings.feedback_url.as_deref(), BUILT_FEEDBACK_URL);
    bugreport::route(
        session(lobby).map(|(gateway, _)| gateway).as_deref(),
        service.as_deref(),
    )
}

/// The session the report is sent with, and the gateway it belongs to.
fn session(lobby: Option<&LobbyState>) -> Option<(String, String)> {
    let lobby = lobby?;
    if lobby.offline.is_some() {
        return None;
    }
    let token = lobby.lobby.token()?;
    Some((lobby.gateway.clone(), token.to_string()))
}

/// Everywhere a token lives, for [`keyring`]: read the same way by the
/// form's Send and by the crash courier.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Holders<'w> {
    lobby: Option<Res<'w, LobbyState>>,
    host: Option<Res<'w, crate::InstalledHost>>,
    prefs: Option<Res<'w, crate::prefs::Prefs>>,
    text: Option<Res<'w, crate::cardtext::TextGateway>>,
}

impl Holders<'_> {
    /// Every token they hold, and the settings' guests.
    fn keyring(&self, settings: &ClientSettings) -> Keyring {
        keyring(
            self.lobby.as_deref(),
            self.host.as_deref(),
            self.prefs.as_deref(),
            self.text.as_deref(),
            settings,
        )
    }
}

/// Every token this client holds that a report must not carry, from every
/// place it lives and whatever screen is up (#314): the lobby's session and
/// the copies of it the preferences and the card text keep, the seat token
/// the lobby holds between a join and the table and the one the table's
/// host dials with after that, and every guest kept on this device.
pub(crate) fn keyring(
    lobby: Option<&LobbyState>,
    host: Option<&crate::InstalledHost>,
    prefs: Option<&crate::prefs::Prefs>,
    text: Option<&crate::cardtext::TextGateway>,
    settings: &ClientSettings,
) -> Keyring {
    let mut ring = Keyring::default();
    let owned = |token: &str| token.to_string();
    ring.sessions
        .extend(lobby.and_then(|l| l.lobby.token()).map(owned));
    ring.sessions.extend(
        prefs
            .and_then(crate::prefs::Prefs::session_token)
            .map(owned),
    );
    ring.sessions.extend(
        text.and_then(|t| t.0.as_ref())
            .map(|signed| signed.token.clone()),
    );
    ring.seats.extend(
        lobby
            .and_then(|l| l.lobby.awaiting())
            .map(|seat| seat.seat_token.clone()),
    );
    if let Some(Screen::Seated(seat)) = lobby.map(|l| l.lobby.screen()) {
        ring.seats.push(seat.seat_token.clone());
    }
    ring.seats
        .extend(host.and_then(|h| h.0.seat_token()).map(owned));
    ring.guests
        .extend(settings.guests.values().map(|guest| guest.token.clone()));
    ring
}

/// The device id direct reports go under: the one the settings keep, or a
/// fresh one from the platform's generator, kept from now on.
fn device_id(settings: &mut ClientSettings) -> String {
    let id = bugreport::kept_device_id(settings.report_device.as_deref(), || {
        let mut random = [0u8; 16];
        // A generator that fails leaves zeros, which is still an id; the
        // reports of such a device merely share one.
        let _ = getrandom::fill(&mut random);
        random
    });
    if settings.report_device.as_deref() != Some(id.as_str()) {
        settings.report_device = Some(id.clone());
        settings.save();
    }
    id
}

/// Send was pressed: the confirmation comes up when the report needs one
/// ([`ReportForm::ask_to_send`]), else it goes.
fn ask_to_send(
    desk: &mut ReportDesk,
    holders: &Holders,
    settings: &mut ClientSettings,
    answers: &Answers,
) {
    let route = route(holders.lobby.as_deref(), settings);
    if !route.sends() {
        return;
    }
    let device = settings.report_device.clone().unwrap_or_default();
    let via = if route.is_direct() {
        Via::Direct { device: &device }
    } else {
        Via::Gateway
    };
    let gathered = desk.gathered.clone();
    if desk.form.ask_to_send(&gathered, &settings.reports, via) {
        send(desk, holders, settings, answers);
    }
}

/// The keyboard's Send (`KEYBOARD.md` W9 step 3): what [`form::typing`]
/// asked for, through the same two doors the buttons take.
fn send_by_key(
    mut desk: ResMut<ReportDesk>,
    holders: Holders,
    mut settings: ResMut<ClientSettings>,
    answers: Res<Answers>,
) {
    let Some(press) = desk.send_by_key.take() else {
        return;
    };
    let desk = desk.as_mut();
    match press {
        form::DeskPress::Send => ask_to_send(desk, &holders, &mut settings, &answers),
        form::DeskPress::ConfirmSend => send(desk, &holders, &mut settings, &answers),
        _ => {}
    }
}

/// Sends the form's report: sealed here, answered into [`Answers`].
fn send(
    desk: &mut ReportDesk,
    holders: &Holders,
    settings: &mut ClientSettings,
    answers: &Answers,
) {
    let keyring = holders.keyring(settings);
    let gathered = desk.gathered.clone();
    match route(holders.lobby.as_deref(), settings) {
        Route::Gateway(_) => {
            let Some((gateway, token)) = session(holders.lobby.as_deref()) else {
                return;
            };
            let consent = settings.reports.clone();
            let Some(json) =
                desk.form
                    .prepare(&gathered, &consent, &keyring.secrets(), Via::Gateway)
            else {
                return;
            };
            post(
                &format!("{}/reports", gateway.trim_end_matches('/')),
                Some(&token),
                json,
                answers,
                Answer::Form,
            );
        }
        Route::Direct(url) => {
            let device = device_id(settings);
            let consent = settings.reports.clone();
            let via = Via::Direct { device: &device };
            let Some(json) = desk
                .form
                .prepare(&gathered, &consent, &keyring.secrets(), via)
            else {
                return;
            };
            post(&url, None, json, answers, Answer::Form);
        }
        Route::Nowhere => {}
    }
}

/// `POST` a report to `url`, with the session when there is one (a
/// gateway's), its answer posted back.
fn post(
    url: &str,
    token: Option<&str>,
    body: String,
    answers: &Answers,
    answer: impl FnOnce(u16, String) -> Answer + Send + 'static,
) {
    let mut request = ehttp::Request::post(url, body.into_bytes());
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }
    let slot = Arc::clone(&answers.0);
    crate::transport::fetch(request, move |result| {
        let (status, body) = match result {
            Ok(response) => (
                response.status,
                response.text().unwrap_or_default().to_string(),
            ),
            Err(_) => (0, String::new()),
        };
        if let Ok(mut slot) = slot.lock() {
            slot.push(answer(status, body));
        }
    });
}

/// Hands the gateway's answers to the form and the courier.
fn answers(answers: Res<Answers>, mut desk: ResMut<ReportDesk>) {
    let drained: Vec<Answer> = answers
        .0
        .lock()
        .map(|mut slot| slot.drain(..).collect())
        .unwrap_or_default();
    for answer in drained {
        match answer {
            Answer::Form(status, body) => desk.form.answered(status, &body),
            Answer::Crash(status) => {
                // Received, or refused for good: either way this crash is
                // done. Anything else keeps the file for the next start.
                if matches!(status, 200 | 201 | 400 | 413) {
                    crate::settings::store::remove_named(CRASH_FILE);
                }
                desk.crash = None;
            }
        }
    }
}

// ------------------------------------------------------------------ crashes

/// Writes a crash down where the next start will find it (#310).
///
/// Installed first thing by [`crate::standalone::run`], and chained to the
/// hook that was there, so the panic is still printed. Nothing here touches
/// the network or the world: a file, and the message in it.
pub(crate) fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic".to_string());
        let file = CrashFile {
            gateway: SIGNED_IN_AT.lock().ok().and_then(|g| g.clone()),
            build: build(),
            record: bugreport::CrashRecord {
                message: bugreport::scrub_home(&message, &home),
                location: info
                    .location()
                    .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())),
                // Forced, because `RUST_BACKTRACE` is unset on a player's
                // machine and the default capture would say "disabled".
                // Scrubbed and cut in client-core, where it is tested.
                backtrace: bugreport::bounded_backtrace(
                    &std::backtrace::Backtrace::force_capture().to_string(),
                    &home,
                ),
                at_unix: web_time::SystemTime::now()
                    .duration_since(web_time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
                platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
                thread: std::thread::current().name().map(str::to_string),
            },
        };
        crate::settings::store::write_named(CRASH_FILE, &file.to_text());
        previous(info);
    }));
}

/// Keeps [`SIGNED_IN_AT`] on the gateway the lobby is signed in to.
fn remember_the_gateway(lobby: Option<Res<LobbyState>>, mut last: Local<Option<String>>) {
    let now = session(lobby.as_deref()).map(|(gateway, _)| gateway);
    if *last != now {
        if let Ok(mut slot) = SIGNED_IN_AT.lock() {
            slot.clone_from(&now);
        }
        *last = now;
    }
}

/// At start: a crash file, and what the player said to do with one.
fn find_a_crash(mut desk: ResMut<ReportDesk>, settings: Res<ClientSettings>) {
    let text = crate::settings::store::read_named(CRASH_FILE);
    if meet_the_crash(&mut desk, settings.reports.crashes, text.as_deref()) {
        crate::settings::store::remove_named(CRASH_FILE);
    }
}

/// What start does with the crash file's `text` under `consent`: holds the
/// crash for the courier, or asks the question. `true` when the file is to
/// be deleted: answered "never", or torn by the crash it was written in.
fn meet_the_crash(desk: &mut ReportDesk, consent: CrashConsent, text: Option<&str>) -> bool {
    let file = text.and_then(CrashFile::from_text);
    let torn = text.is_some() && file.is_none();
    match bugreport::crash_step(consent, file.is_some()) {
        CrashStep::Nothing => torn,
        CrashStep::Discard => true,
        CrashStep::Ask => {
            desk.crash = file;
            desk.asking = true;
            false
        }
        CrashStep::Send => {
            desk.crash = file;
            false
        }
    }
}

/// The crash question's answer, from the form's buttons.
fn answer_the_crash_question(desk: &mut ReportDesk, settings: &mut ClientSettings, send: bool) {
    desk.asking = false;
    desk.swallow = true;
    settings.reports.crashes = if send {
        CrashConsent::Send
    } else {
        CrashConsent::Never
    };
    settings.save();
    if !send {
        desk.crash = None;
        crate::settings::store::remove_named(CRASH_FILE);
    }
}

/// Sends a waiting crash once the lobby is signed in where it belongs.
fn send_the_crash(
    mut desk: ResMut<ReportDesk>,
    holders: Holders,
    settings: Res<ClientSettings>,
    answers: Res<Answers>,
) {
    if desk.crash_tried || desk.asking || settings.reports.crashes != CrashConsent::Send {
        return;
    }
    let Some(file) = desk.crash.clone() else {
        return;
    };
    let Some((gateway, token)) = session(holders.lobby.as_deref()) else {
        return;
    };
    if !file.sends_to(&gateway, gateway_list::PINNED) {
        return;
    }
    desk.crash_tried = true;
    let system = settings
        .reports
        .allows(Category::System)
        .then(|| system(None, None, &settings.lang));
    let keyring = holders.keyring(&settings);
    if let Ok((json, _)) = bugreport::crash_submission(&file, system).sealed(&keyring.secrets()) {
        post(
            &format!("{}/reports", gateway.trim_end_matches('/')),
            Some(&token),
            json,
            &answers,
            |status, _| Answer::Crash(status),
        );
    }
}

/// The lobby's "report a problem" button, beside the music controls.
pub(crate) fn button(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    metrics: crate::lobby::Metrics,
    lang: Lang,
) -> Entity {
    let id = crate::lobby::button(
        commands,
        fonts,
        metrics,
        baylee_client_core::i18n::Phrase::ReportButton.text(lang),
        crate::lobby::Press::Shared(crate::lobby::SharedPress::PickerNothing),
        crate::hud::palette::PANEL,
        true,
    );
    commands
        .entity(id)
        .remove::<crate::lobby::Press>()
        .insert(Button)
        .observe(
            |mut click: On<Pointer<Click>>, mut desk: ResMut<ReportDesk>| {
                click.propagate(false);
                desk.asked = true;
            },
        );
    id
}

/// The consent this device keeps, as the form edits it.
fn consent_mut(settings: &mut ClientSettings) -> &mut Consent {
    &mut settings.reports
}
