//! The terms sheet (WG-1, `DESIGN-v5` §11, `KEYBOARD.md` §7.9): asking the
//! gateway, the sheet itself, and its keys.
//!
//! The decisions are `baylee_client_core::terms`; this is the wire and the
//! drawing. The sheet stands over whatever the lobby shows after a sign-in
//! (it is modal, its own `TabOrder`), and holds the screen until answered:
//! **Accept and continue** (enabled once the end of the text has been in
//! view), **Not now**, which signs out with nothing stored — a guest is
//! asked first — or **Decline and delete account**, which opens the
//! account deletion's own confirmation over the sheet (#292: the password
//! again for an account, none for a guest). Esc moves focus to Not now and
//! does nothing else; in the confirmation it is the confirmation's Cancel.

use std::sync::Arc;

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::terms::{Ask, Block, Sheet, TermsDoc, must_ask};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

use super::super::clicks::sign_out;
use super::super::{
    FrontPress, List, LobbyState, Mailbox, Press, Reply, Scrollable, Scrolled, http,
};
use super::keys::TERMS;
use crate::hud::{UiFonts, tf, tf_bold, tf_italic};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::Stop;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::tokens;

/// What the gateway answered about the terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TermsReply {
    /// A sign-in's `terms_stale`, read off its answer before its event.
    Stale(bool),
    /// `GET /terms`, to a question asked in `asked`.
    Doc {
        /// The language the text was asked in (not the one it came in: a
        /// gateway without that language answers in English).
        asked: Lang,
        /// What came back.
        doc: TermsDoc,
    },
    /// `GET /terms` failed (or the gateway has no terms after all).
    FetchFailed,
    /// `POST /account/terms` recorded this version.
    Accepted(String),
    /// `409`: the text changed while the sheet was up.
    Changed,
    /// The acceptance failed some other way; the gateway's words.
    SendFailed(String),
}

/// Where a sheet wants the keyboard's focus once its tree is drawn: a stop
/// of the terms or the About sheet, by name (Esc → Not now; the text on
/// opening). Kept until the stop is drawn, since the sheet is rebuilt
/// after the key that asked.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SheetFocus(pub(crate) Option<&'static str>);

/// Moves focus where [`SheetFocus`] asks, once that stop is drawn, and
/// gives a sheet that has just opened its first focus (the text).
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(crate) fn place_sheet_focus(
    state: Res<LobbyState>,
    stops: Query<(Entity, &Stop)>,
    mut wanted: ResMut<SheetFocus>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    mut remembered: ResMut<crate::shellkit::focus::Remembered>,
    mut kept: ResMut<crate::lobby::focusing::Kept>,
    mut was: Local<(bool, bool, bool)>,
) {
    // Decline's confirmation stands over the sheet and takes the keyboard
    // (its password box, Enter, Esc, Tab; `keyboard`): no stop of the sheet
    // keeps the focus under it, or Enter would press that stop too, and
    // none is restored while it stands. Cancelled, focus is back on Decline.
    let deleting = state.terms.up() && state.lobby.deleting_account().is_some();
    if deleting != was.2 {
        was.2 = deleting;
        if !deleting && state.terms.up() {
            wanted.0 = Some("decline");
        }
    }
    if deleting {
        if focus.get().is_some() {
            focus.clear();
        }
        if remembered.0.is_some() {
            remembered.0 = None;
        }
        return;
    }
    let table = if state.terms.up() {
        Some(TERMS.name)
    } else if state.about_open {
        Some(super::keys::ABOUT.name)
    } else {
        None
    };
    let reading = matches!(state.terms.sheet(), Sheet::Reading(_));
    let now = (table.is_some(), reading);
    if now != (was.0, was.1) {
        // Opened, or its text arrived: focus on the text; while the terms
        // are still on their way (or failed), on the sheet's one answer.
        if table.is_some() {
            wanted.0 = Some(if reading || state.about_open {
                "text"
            } else if matches!(state.terms.sheet(), Sheet::Failed) {
                "retry"
            } else {
                "not-now"
            });
        }
        (was.0, was.1) = now;
    }
    let (Some(table), Some(id)) = (table, wanted.0) else {
        if table.is_none() && wanted.0.is_some() {
            wanted.0 = None;
        }
        return;
    };
    let Some((entity, stop)) = stops.iter().find(|(_, s)| s.table == table && s.id == id) else {
        return;
    };
    if focus.get() != Some(entity) {
        focus.set(entity, FocusCause::Navigated);
    }
    // Esc shows the ring (a key moved focus); opening does not.
    visible.0 = id != "text";
    remembered.0 = Some(*stop);
    // And the lobby's own memory of the ring, at once (`focusing::Kept`): a
    // rebuild in this frame would otherwise give the ring back to the stop
    // that had it before (Esc landed on the text again, depending on how
    // the schedule happened to order the systems).
    kept.keep(*stop);
    wanted.0 = None;
}

/// The text region's scroll container.
#[derive(Component)]
pub(crate) struct TermsText;

/// The progress hairline under the text.
#[derive(Component)]
pub(crate) struct TermsProgress;

/// Reads `terms_stale` out of a sign-in's answer, if it has one.
#[must_use]
pub(crate) fn stale_of(body: &[u8]) -> Option<bool> {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()?
        .get("terms_stale")?
        .as_bool()
}

/// `GET /terms?lang=` (public), in the interface's language.
fn fetch(gateway: &str, lang: Lang, epoch: u64, mailbox: &Mailbox) {
    // No address, nothing to ask (a headless test's lobby).
    if gateway.is_empty() {
        return;
    }
    let url = baylee_client_core::terms::url(gateway, lang);
    let box_ = Arc::clone(&mailbox.0);
    crate::transport::fetch(ehttp::Request::get(url), move |answer| {
        let reply = match answer {
            Ok(response) if response.ok => serde_json::from_slice::<TermsDoc>(&response.bytes)
                .map_or(TermsReply::FetchFailed, |doc| TermsReply::Doc {
                    asked: lang,
                    doc,
                }),
            _ => TermsReply::FetchFailed,
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(epoch, Box::new(Reply::Terms(reply))));
        }
    });
}

/// `POST /account/terms {version}` with the session.
fn accept(gateway: &str, token: &str, version: &str, epoch: u64, lang: Lang, mailbox: &Mailbox) {
    if gateway.is_empty() {
        return;
    }
    let url = format!("{}/account/terms", gateway.trim_end_matches('/'));
    // The lobby's own JSON request (its `Content-Type` is what the gateway
    // reads the body by), signed with the session.
    let request = http::bearer(
        http::json_post(&url, &serde_json::json!({ "version": version })),
        Some(token),
    );
    let box_ = Arc::clone(&mailbox.0);
    let version = version.to_string();
    crate::transport::fetch(request, move |answer| {
        let reply = match answer {
            Ok(response) if response.ok => Reply::Terms(TermsReply::Accepted(version)),
            Ok(response) if response.status == 401 => Reply::Expired,
            Ok(response) if response.status == 409 => Reply::Terms(TermsReply::Changed),
            Ok(response) => {
                Reply::Terms(TermsReply::SendFailed(http::gateway_error(lang, &response)))
            }
            Err(err) => Reply::Terms(TermsReply::SendFailed(
                Phrase::GatewayNoAnswer.fill(lang, &[&err]),
            )),
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(epoch, Box::new(reply)));
        }
    });
}

/// Carries out what the sheet asked.
pub(in crate::lobby) fn perform(
    ask: Ask,
    state: &mut ResMut<LobbyState>,
    prefs: &mut ResMut<crate::prefs::Prefs>,
    scrolled: &mut ResMut<Scrolled>,
    mailbox: &Mailbox,
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
) {
    match ask {
        Ask::Fetch(lang) => {
            scrolled.set(List::Terms, 0.0);
            fetch(&state.gateway, lang, state.gateway_epoch, mailbox);
        }
        Ask::Accept(version) => {
            if let Some(token) = state.lobby.token().map(str::to_string) {
                accept(
                    &state.gateway,
                    &token,
                    &version,
                    state.gateway_epoch,
                    state.lobby.lang(),
                    mailbox,
                );
            }
        }
        Ask::SignOut => sign_out(state, prefs, scrolled, mailbox, settings),
    }
}

/// An answer about the terms, from the mailbox.
pub(in crate::lobby) fn receive(
    reply: TermsReply,
    state: &mut ResMut<LobbyState>,
    mailbox: &Mailbox,
    settings: &mut Option<ResMut<crate::settings::ClientSettings>>,
) {
    match reply {
        TermsReply::Stale(stale) => state.terms_stale = Some(stale),
        TermsReply::Doc { asked, doc } => state.terms.loaded(doc, asked),
        TermsReply::FetchFailed => state.terms.failed(),
        TermsReply::Accepted(version) => {
            if state.terms.accepted().is_some() {
                let gateway = state.gateway.clone();
                if let Some(settings) = settings.as_mut()
                    && settings.terms.get(&gateway) != Some(&version)
                {
                    settings.terms.insert(gateway, version);
                    settings.save();
                }
            }
        }
        TermsReply::Changed => {
            if let Ask::Fetch(lang) = state.terms.changed() {
                fetch(&state.gateway, lang, state.gateway_epoch, mailbox);
            }
        }
        TermsReply::SendFailed(said) => {
            state.terms.send_failed();
            state.lobby.say(said);
        }
    }
}

/// Follows the session: a sign-in that landed asks the sheet when the
/// gateway (or, for a kept guest, `/info` against this device's copy) says
/// so; signing out takes the sheet down.
pub(in crate::lobby) fn follow_the_session(
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    mut last: Local<Option<String>>,
) {
    let token = state.lobby.token().map(str::to_string);
    if *last == token {
        return;
    }
    let signed_in = last.is_none() && token.is_some();
    *last = token;
    if !signed_in {
        if state.terms.up() {
            state.terms.close();
        }
        if state.terms_stale.is_some() {
            state.terms_stale = None;
        }
        return;
    }
    if state.lobby.offline() {
        return;
    }
    let stale = state.terms_stale.take();
    let current = match state.probes.get(&state.gateway) {
        Some(baylee_client_core::lobby::gateway_info::Probe::Known(info)) => info.terms.clone(),
        _ => None,
    };
    let gateway = state.gateway.clone();
    let here = settings
        .as_ref()
        .and_then(|s| s.terms.get(&gateway).cloned());
    if must_ask(stale, current.as_deref(), here.as_deref()) {
        let lang = state.lobby.lang();
        let ask = state.terms.ask(lang);
        perform(
            ask,
            &mut state,
            &mut prefs,
            &mut scrolled,
            &mailbox,
            &mut settings,
        );
    } else if stale == Some(false)
        && let Some(version) = current
        && here.as_ref() != Some(&version)
        && let Some(settings) = settings.as_mut()
    {
        // The account has accepted these already: the device's copy
        // follows, for a later return as a kept guest.
        settings.terms.insert(gateway, version);
        settings.save();
    }
}

/// Follows the interface's language: a switch while the sheet is up asks
/// for the text again in the new one (`terms::Terms::relang`).
pub(in crate::lobby) fn follow_the_language(
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
) {
    let lang = state.lobby.lang();
    // Read without `DerefMut` first: a sheet that is down, up in this
    // language already, or sending its acceptance, is nearly every frame,
    // and must not mark the lobby changed.
    let sending = matches!(state.terms.sheet(), Sheet::Reading(r) if r.sending);
    if !state.terms.up() || state.terms.lang() == lang || sending {
        return;
    }
    if let Some(Ask::Fetch(lang)) = state.terms.relang(lang) {
        scrolled.set(List::Terms, 0.0);
        fetch(&state.gateway, lang, state.gateway_epoch, &mailbox);
    }
}

/// The text region's reading: the end in view enables Accept, and the
/// hairline under it shows how far down the reader is.
pub(crate) fn read_the_terms(
    texts: Query<(&ScrollPosition, &ComputedNode), With<TermsText>>,
    mut bars: Query<&mut Node, With<TermsProgress>>,
    mut state: ResMut<LobbyState>,
) {
    let Ok((at, node)) = texts.single() else {
        return;
    };
    let scale = node.inverse_scale_factor();
    let seen = node.size().y * scale;
    let whole = node.content_size().y * scale;
    if whole <= 0.0 {
        return;
    }
    let share = ((at.y + seen) / whole).clamp(0.0, 1.0);
    for mut bar in &mut bars {
        let width = Val::Percent(share * 100.0);
        if bar.width != width {
            bar.width = width;
        }
    }
    let read = matches!(state.terms.sheet(), Sheet::Reading(r) if r.read_to_end);
    if !read && at.y + seen >= whole - 2.0 && matches!(state.terms.sheet(), Sheet::Reading(_)) {
        state.terms.reached_end();
    }
}

/// The sheet's keys (`KEYBOARD.md` §7.9): Space / `PageDown`, `PageUp`, ↑↓,
/// Home and End scroll the text while it has focus (End enables Accept);
/// Esc moves focus to Not now (or answers a guest's question with Stay) and
/// does nothing else; Enter on a focused button is the walker's.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(in crate::lobby) fn terms_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut state: ResMut<LobbyState>,
    mut texts: Query<(&mut ScrollPosition, &ComputedNode), With<TermsText>>,
    stops: Query<(Entity, &Stop)>,
    focus: Res<InputFocus>,
    mut wanted: ResMut<SheetFocus>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
) {
    // Decline's confirmation over the sheet answers its own keys.
    if !state.terms.up() || state.lobby.deleting_account().is_some() {
        keys.clear();
        return;
    }
    let on_text = focus
        .get()
        .and_then(|f| stops.get(f).ok())
        .is_some_and(|(_, s)| s.table == TERMS.name && s.id == "text");
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        if key.logical_key == Key::Escape {
            state.terms.escape();
            wanted.0 = Some("not-now");
            continue;
        }
        if !on_text {
            continue;
        }
        // Enter in the text accepts once Accept works (§7.9).
        if key.logical_key == Key::Enter {
            if let Some(Ask::Accept(version)) = state.terms.accept()
                && let Some(token) = state.lobby.token().map(str::to_string)
            {
                accept(
                    &state.gateway,
                    &token,
                    &version,
                    state.gateway_epoch,
                    state.lobby.lang(),
                    &mailbox,
                );
            }
            continue;
        }
        // End is the end of the text whatever is laid out yet.
        if key.logical_key == Key::End {
            state.terms.reached_end();
        }
        let Ok((mut at, node)) = texts.single_mut() else {
            continue;
        };
        let scale = node.inverse_scale_factor();
        let page = node.size().y * scale;
        let end = (node.content_size().y * scale - page).max(0.0);
        let to = match &key.logical_key {
            Key::Space | Key::PageDown => at.y + page * 0.9,
            Key::PageUp => at.y - page * 0.9,
            Key::ArrowDown => at.y + 40.0,
            Key::ArrowUp => at.y - 40.0,
            Key::Home => 0.0,
            Key::End => end,
            _ => continue,
        }
        .clamp(0.0, end);
        if (at.y - to).abs() > f32::EPSILON {
            at.y = to;
            scrolled.set(List::Terms, to);
        }
    }
}

/// The words of one block as text spans under a paragraph node.
fn block_text(
    commands: &mut Commands,
    fonts: &UiFonts,
    size: f32,
    spans: &[baylee_client_core::terms::Span],
    bold_all: bool,
) -> Entity {
    let root = commands
        .spawn((
            Text::new(""),
            tf(fonts, size),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    for span in spans {
        let font = if span.bold || bold_all {
            tf_bold(fonts, size)
        } else if span.italic {
            tf_italic(fonts, size)
        } else {
            tf(fonts, size)
        };
        let child = commands
            .spawn((
                TextSpan::new(span.text.clone()),
                font,
                TextColor(tokens::INK),
            ))
            .id();
        commands.entity(root).add_child(child);
    }
    root
}

/// The text as nodes.
pub(crate) fn document(commands: &mut Commands, kit: Kit, blocks: &[Block]) -> Vec<Entity> {
    let mut out = Vec::new();
    for block in blocks {
        let node = match block {
            Block::Heading { level, spans } => {
                let size = if *level == 1 {
                    kit.m.head
                } else {
                    kit.m.text * 1.08
                };
                let text = block_text(commands, kit.fonts, size, spans, *level > 1);
                commands.entity(text).insert(Node {
                    margin: UiRect::top(kit.m.px(if *level == 1 { 0.0 } else { 6.0 })),
                    ..default()
                });
                text
            }
            Block::Paragraph(spans) => block_text(commands, kit.fonts, kit.m.text, spans, false),
            Block::Item { number, spans } => {
                let row = commands
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            column_gap: kit.m.px(8.0),
                            padding: UiRect::left(kit.m.px(8.0)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                let mark = commands
                    .spawn((
                        Text::new(
                            number.map_or_else(|| "\u{2022}".to_string(), |n| format!("{n}.")),
                        ),
                        tf(kit.fonts, kit.m.text),
                        TextColor(tokens::MUTED),
                        Node {
                            min_width: kit.m.px(18.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                let words = block_text(commands, kit.fonts, kit.m.text, spans, false);
                commands.entity(words).insert(Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: px_fixed(0.0),
                    min_width: px_fixed(0.0),
                    ..default()
                });
                commands.entity(row).add_children(&[mark, words]);
                row
            }
            Block::Rule => commands
                .spawn((
                    Node {
                        height: px_fixed(1.0),
                        width: Val::Percent(100.0),
                        margin: UiRect::vertical(kit.m.px(4.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(tokens::BORDER),
                    Pickable::IGNORE,
                ))
                .id(),
        };
        out.push(node);
    }
    out
}

/// A stop on the terms sheet.
fn at(id: &'static str) -> Stop {
    Stop::new(TERMS.name, id)
}

/// The terms sheet over `root`, while it is up.
#[allow(clippy::too_many_lines)] // one sheet, read top to bottom
pub(crate) fn sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    if !state.terms.up() {
        return;
    }
    let lang = state.lobby.lang();
    let name = super::super::gateway::title_of(state, &state.gateway);
    let phone = kit.m.frame == crate::shellkit::Frame::Phone;
    let mut body: Vec<Entity> = Vec::new();
    let mut footer: Vec<Entity> = Vec::new();
    let caption_text = match state.terms.sheet() {
        Sheet::Reading(r) => {
            let mut said = Phrase::TermsVersion.fill(lang, &[&r.doc.version]);
            if let Some(updated) = &r.doc.updated {
                said.push_str(" \u{b7} ");
                said.push_str(&Phrase::TermsUpdated.fill(lang, &[updated]));
            }
            said
        }
        _ => String::new(),
    };
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: if phone {
                    Val::Percent(100.0)
                } else {
                    kit.m.px(720.0)
                },
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(if phone {
                    0.0
                } else {
                    tokens::RADIUS_PANEL
                })),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            // A press on the sheet's own ground is no press behind it.
            Press::Shared(super::super::SharedPress::PickerNothing),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(Phrase::TermsTitle.fill(lang, &[&name])),
            tf(kit.fonts, kit.m.h1),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    body.push(title);
    if !caption_text.is_empty() {
        let caption = commands
            .spawn((
                Text::new(caption_text),
                tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        body.push(caption);
    }
    match state.terms.sheet() {
        Sheet::Closed => {}
        Sheet::Fetching => {
            let words = crate::shellkit::surfaces::prose(
                commands,
                kit,
                Phrase::TermsLoading.text(lang),
                true,
            );
            body.push(words);
            let not_now = controls::button(
                commands,
                kit,
                Phrase::TermsNotNow.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                (Press::Front(FrontPress::TermsNotNow), at("not-now")),
            );
            footer.push(not_now);
        }
        Sheet::Failed => {
            let retry = controls::button(
                commands,
                kit,
                Phrase::TermsRetry.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                (Press::Front(FrontPress::TermsRetry), at("retry")),
            );
            let line = crate::shellkit::states::error_line(
                commands,
                kit,
                Phrase::TermsLoadFailed.text(lang),
                retry,
            );
            body.push(line);
            let not_now = controls::button(
                commands,
                kit,
                Phrase::TermsNotNow.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                (Press::Front(FrontPress::TermsNotNow), at("not-now")),
            );
            footer.push(not_now);
        }
        Sheet::Reading(reading) => {
            let text = commands
                .spawn((
                    Role::Scroll,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        min_height: px_fixed(0.0),
                        max_height: kit.m.px(if phone { 1000.0 } else { 420.0 }),
                        padding: UiRect::all(kit.m.px(16.0)),
                        row_gap: kit.m.px(10.0),
                        overflow: Overflow::scroll_y(),
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
                    BorderColor::all(tokens::BORDER),
                    ScrollPosition(Vec2::new(0.0, scrolled.get(List::Terms))),
                    Scrollable(List::Terms),
                    TermsText,
                    at("text"),
                    Pickable::default(),
                ))
                .id();
            let blocks = document(commands, kit, &reading.blocks);
            commands.entity(text).add_children(&blocks);
            body.push(text);
            let track = commands
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: px_fixed(2.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(tokens::BORDER),
                    Pickable::IGNORE,
                ))
                .id();
            let bar = commands
                .spawn((
                    TermsProgress,
                    Node {
                        width: Val::Percent(if reading.read_to_end { 100.0 } else { 0.0 }),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(tokens::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(track).add_child(bar);
            body.push(track);
            let note = crate::shellkit::surfaces::prose(
                commands,
                kit,
                Phrase::TermsStoredNote.text(lang),
                true,
            );
            body.push(note);
            if reading.asking_guest {
                let ask = crate::shellkit::surfaces::prose(
                    commands,
                    kit,
                    Phrase::TermsGuestAsk.text(lang),
                    false,
                );
                body.push(ask);
                let stay = controls::button(
                    commands,
                    kit,
                    Phrase::TermsStay.text(lang),
                    Weight::Primary,
                    Live::Yes,
                    None,
                    (Press::Front(FrontPress::TermsStay), at("stay")),
                );
                let leave = controls::button(
                    commands,
                    kit,
                    Phrase::TermsSignOut.text(lang),
                    Weight::Danger,
                    Live::Yes,
                    None,
                    (Press::Front(FrontPress::TermsNotNow), at("leave")),
                );
                footer.extend([stay, leave]);
            } else {
                let not_now = controls::button(
                    commands,
                    kit,
                    Phrase::TermsNotNow.text(lang),
                    Weight::Secondary,
                    Live::Yes,
                    None,
                    (Press::Front(FrontPress::TermsNotNow), at("not-now")),
                );
                let live = if state.terms.can_accept() {
                    Live::Yes
                } else if reading.sending {
                    Live::No(Phrase::TermsSending.text(lang))
                } else {
                    Live::No(Phrase::TermsScrollToAccept.text(lang))
                };
                // Declining is the account's deletion, by its own
                // confirmation (password again for an account, none for a
                // guest); the terms text names this button word for word.
                let decline = controls::button(
                    commands,
                    kit,
                    Phrase::TermsDecline.text(lang),
                    Weight::Danger,
                    if reading.sending {
                        Live::No(Phrase::TermsSending.text(lang))
                    } else {
                        Live::Yes
                    },
                    None,
                    (Press::Front(FrontPress::TermsDecline), at("decline")),
                );
                let accept = controls::button(
                    commands,
                    kit,
                    Phrase::TermsAccept.text(lang),
                    Weight::Primary,
                    live,
                    Some("Enter"),
                    (Press::Front(FrontPress::TermsAccept), at("accept")),
                );
                footer.extend([not_now, decline, accept]);
            }
        }
    }
    let foot = commands
        .spawn((
            Node {
                column_gap: px_fixed(kit.m.gap),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::FlexStart,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px_fixed(kit.m.gap),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(foot).add_children(&footer);
    commands.entity(surface).add_children(&body);
    commands.entity(surface).add_child(foot);
    let scrim = crate::shellkit::surfaces::sheet(commands, surface);
    if phone {
        commands
            .entity(scrim)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.align_items = AlignItems::Stretch;
            });
    }
    commands.entity(root).add_child(scrim);
}
