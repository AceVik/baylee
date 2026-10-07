//! The shell's header and strips over the lobby (the shell design, §2.1,
//! §2.5; WP0b-3): the header on every signed-in screen, the seated strip on
//! every one but the room, the reconnecting strip while the feed is down,
//! and their popovers. What they say is `client_core::lobby::strips`'s;
//! the components are the kit's (`shellkit::header`).
//!
//! On a Phone the seated strip folds into the header as the gold Return
//! pill, in the gateway dot's place, and no strip is drawn (S4-3).

use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::header::{self as kit_header, HeaderActions, HeaderLook, Reach};
use crate::shellkit::surfaces::{self, MenuItem};
use crate::shellkit::{Frame as ShellFrame, ShellMetrics, px_fixed, tokens};
use client_core::lobby::strips::{self, Strip};
use std::fmt::Write as _;

/// A header popover that is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum HeaderMenu {
    /// The gateway pill's: where, which version, how busy.
    Gateway,
    /// The bell's: what happened that the player may have missed.
    Bell,
    /// The account pill's: shortcuts, a report, signing out.
    Account,
}

/// A press on the header or a strip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum HeaderPress {
    /// Nav pill `n`: Play, Decks, Settings.
    Nav(u8),
    /// The gateway pill (or its dot).
    Gateway,
    /// The bell.
    Bell,
    /// The account pill (or its letter).
    Account,
    /// The seated strip's Return, and the Phone header's Return pill.
    Return,
    /// The seated strip's Leave (waiting tables only).
    Leave,
    /// The reconnecting strip's Retry now.
    RetryNow,
    /// The account menu: the `?` overlay.
    Shortcuts,
    /// The account menu: the report form.
    Report,
    /// The account menu: sign out.
    SignOut,
}

/// Something a header press asks of a system the press handler cannot
/// reach (the kit's overlay, the report form).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ShellAsk {
    /// Open the `?` overlay.
    Shortcuts,
    /// Open the report form.
    Report,
}

/// Which nav pill is the screen shown.
fn active(state: &LobbyState) -> Option<usize> {
    if state.settings.is_open() {
        return Some(2);
    }
    match state.lobby.screen() {
        Screen::Table if state.lobby.library().page.is_some() => Some(1),
        Screen::Table => Some(match state.hub {
            Hub::Play => 0,
            Hub::Decks => 1,
        }),
        _ => None,
    }
}

/// The seated strip the lobby calls for, unless the room itself is shown.
pub(super) fn seated(state: &LobbyState) -> Option<Strip> {
    strips::strip(&state.lobby)
        .filter(|_| !strips::in_the_room(&state.lobby, state.settings.is_open()))
}

/// Whether this screen wears the header: every signed-in screen but the
/// builder, which has its own (its breadcrumb), and the front door.
pub(super) fn wears_header(state: &LobbyState) -> bool {
    let signed_in = state.lobby.token().is_some() || state.lobby.offline();
    signed_in && matches!(state.lobby.screen(), Screen::Table)
}

/// The account pill's handle: `GET /me`'s, a kept guest's, else none yet.
fn handle(state: &LobbyState) -> Option<String> {
    if state.lobby.offline() {
        return None;
    }
    state
        .lobby
        .me()
        .map(|me| me.handle.clone())
        .filter(|h| !h.is_empty())
        .or_else(|| state.lobby.kept_guest().map(|g| g.handle.clone()))
        .or_else(|| Some("\u{2026}".to_string()))
}

/// Draws the header (where the screen wears one), the strips and an open
/// popover at the top of `root`.
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    metrics: Metrics,
) {
    let seated = seated(state);
    if wears_header(state) {
        bar(commands, root, state, kit, metrics, seated.is_some());
    }
    strips(commands, root, state, kit, seated);
    // The builder wears a header of its own (`buildui::header`) with the
    // same dot, bell and account, and their popovers are these.
    let builder = matches!(state.lobby.screen(), Screen::Build);
    if let Some(menu) = state.header_menu.filter(|_| wears_header(state) || builder) {
        let popover = popover(commands, state, kit, menu);
        commands.entity(root).add_child(popover);
    }
}

/// The header bar itself, and the quick settings' menu when it is open.
fn bar(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    metrics: Metrics,
    seated: bool,
) {
    let lang = state.lobby.lang();
    {
        let reach = if state.lobby.offline() {
            Reach::Offline
        } else if state.feed_down {
            Reach::Down
        } else {
            Reach::Up
        };
        let gateway = state
            .gateway_name()
            .unwrap_or_else(|| state.gateway.clone());
        let counts = if state.lobby.offline() {
            Phrase::ShellOfflinePill.text(lang).to_string()
        } else {
            strips::pill_words(state.lobby.stats(), &state.lobby, lang)
        };
        let handle = handle(state);
        // The quick settings (language, music: the music's toggle stands on
        // every screen it plays on) keep their gear beside the bell.
        let gear = commands
            .spawn((
                Text::new("\u{f013}"),
                crate::hud::icon_tf(kit.fonts, kit.m.text),
                TextColor(tokens::INK),
            ))
            .id();
        let gear = crate::shellkit::controls::hit(
            commands,
            kit,
            gear,
            Press::Front(FrontPress::FrontMenu),
        );
        let tools = [gear];
        let look = HeaderLook {
            brand: Phrase::AppName.text(lang),
            // The short form (§2.1); the full one is in the gateway popover.
            build: baylee_build::VERSION,
            nav: [
                Phrase::ShellPlay.text(lang),
                Phrase::ShellDecks.text(lang),
                Phrase::Settings.text(lang),
            ],
            active: active(state),
            reach,
            gateway: &gateway,
            counts: &counts,
            unread: state.bell.unread(),
            handle: handle.as_deref(),
            return_pill: seated.then(|| Phrase::ShellReturn.text(lang)),
            tools: &tools,
        };
        let bar = kit_header::header(
            commands,
            kit,
            &look,
            HeaderActions {
                nav: |i| Press::Header(HeaderPress::Nav(u8::try_from(i).unwrap_or(0))),
                gateway: Press::Header(HeaderPress::Gateway),
                bell: Press::Header(HeaderPress::Bell),
                account: Press::Header(HeaderPress::Account),
                back: Press::Header(HeaderPress::Return),
            },
        );
        commands.entity(root).add_child(bar);
        if state.front_menu {
            let menu = super::front::gear_menu(commands, state, kit.fonts, metrics, metrics.pad);
            commands.entity(bar).add_child(menu);
        }
    }
}

/// The strips under the header: seated (not on a Phone, where the header's
/// Return pill stands for it), update required, reconnecting.
fn strips(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    seated: Option<Strip>,
) {
    let lang = state.lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    if let Some(strip) = seated.filter(|_| !phone) {
        let back = controls::button(
            commands,
            kit,
            Phrase::ShellReturn.text(lang),
            Weight::Gold,
            Live::Yes,
            None,
            Press::Header(HeaderPress::Return),
        );
        let mut actions = vec![back];
        if strip.can_leave() {
            actions.push(controls::button(
                commands,
                kit,
                Phrase::ShellLeave.text(lang),
                Weight::Danger,
                Live::Yes,
                None,
                Press::Header(HeaderPress::Leave),
            ));
        }
        let bar = kit_header::strip(commands, kit, &strip.sentence(lang), &actions);
        commands.entity(root).add_child(bar);
    }
    if state.update_required() && matches!(state.lobby.screen(), Screen::Table) {
        let bar = kit_header::strip(commands, kit, Phrase::ShellUpdateRequired.text(lang), &[]);
        commands.entity(root).add_child(bar);
    }
    if state.feed_down && !phone && matches!(state.lobby.screen(), Screen::Table) {
        let retry = controls::button(
            commands,
            kit,
            Phrase::ShellRetryNow.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Header(HeaderPress::RetryNow),
        );
        let bar = kit_header::strip(
            commands,
            kit,
            Phrase::ShellReconnecting.text(lang),
            &[retry],
        );
        commands.entity(root).add_child(bar);
    }
}

/// An open header popover, under the header's right end.
fn popover(commands: &mut Commands, state: &LobbyState, kit: Kit, menu: HeaderMenu) -> Entity {
    let lang = state.lobby.lang();
    let surface = match menu {
        // Offline there is no account to sign out of and no gateway to
        // describe: the pill's menu is the way back to the gateways.
        HeaderMenu::Gateway if state.lobby.offline() => surfaces::menu(
            commands,
            kit,
            [MenuItem {
                text: Phrase::ShellSwitchGateway.text(lang),
                keys: None,
                destructive: false,
                action: Press::Header(HeaderPress::SignOut),
            }],
        ),
        HeaderMenu::Gateway => {
            let name = state
                .gateway_name()
                .unwrap_or_else(|| state.gateway.clone());
            let client = format!(
                "{} {}",
                Phrase::ShellThisClient.text(lang),
                baylee_build::short()
            );
            let counts = state.lobby.stats().map_or_else(
                || strips::pill_words(None, &state.lobby, lang),
                |s| {
                    Phrase::ShellStatsLine.fill(
                        lang,
                        &[
                            &s.players_online.to_string(),
                            &s.tables_waiting.to_string(),
                            &s.games_running.to_string(),
                        ],
                    )
                },
            );
            let lines = [name.as_str(), state.gateway.as_str(), &client, &counts];
            surfaces::popover(commands, kit, &lines)
        }
        HeaderMenu::Bell => {
            let said: Vec<String> = state.bell.lines(lang);
            let lines: Vec<&str> = if said.is_empty() {
                vec![Phrase::ShellBellEmpty.text(lang)]
            } else {
                said.iter().map(String::as_str).collect()
            };
            surfaces::popover(commands, kit, &lines)
        }
        HeaderMenu::Account => surfaces::menu(
            commands,
            kit,
            [
                MenuItem {
                    text: Phrase::ShellKeyOverlay.text(lang),
                    keys: Some("?"),
                    destructive: false,
                    action: Press::Header(HeaderPress::Shortcuts),
                },
                MenuItem {
                    text: Phrase::ShellReportProblem.text(lang),
                    keys: Some("F8"),
                    destructive: false,
                    action: Press::Header(HeaderPress::Report),
                },
                MenuItem {
                    text: Phrase::ShellSignOut.text(lang),
                    keys: None,
                    destructive: true,
                    action: Press::Header(HeaderPress::SignOut),
                },
            ],
        ),
    };
    let holder = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(kit.m.header + kit.m.gap),
                right: px_fixed(kit.m.body),
                ..default()
            },
            GlobalZIndex(tokens::z::POPOVER),
        ))
        .id();
    commands.entity(holder).add_child(surface);
    holder
}

/// The kit's measures for the lobby's window, at the device's text step and
/// input class.
pub(super) fn kit_metrics(
    width: f32,
    height: f32,
    step: crate::shellkit::TextSize,
    input: crate::shellkit::InputClass,
) -> ShellMetrics {
    ShellMetrics::of(
        crate::shellkit::Viewport {
            width,
            height,
            platform: crate::shellkit::Platform::current(),
            input,
        },
        step,
    )
}

impl HeaderPress {
    /// What a press on the header or a strip does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        match self {
            HeaderPress::Gateway => toggle(cx.state, HeaderMenu::Gateway),
            HeaderPress::Account => toggle(cx.state, HeaderMenu::Account),
            HeaderPress::Bell => {
                toggle(cx.state, HeaderMenu::Bell);
                if cx.state.bell.unread() > 0 {
                    cx.state.bell.read_all();
                }
            }
            HeaderPress::Shortcuts => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Shortcuts);
            }
            HeaderPress::Report => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Report);
            }
            HeaderPress::SignOut => {
                cx.state.header_menu = None;
                HubPress::SignOut.handle(cx);
            }
            HeaderPress::Nav(2) => {
                if cx.state.library_open() {
                    cx.state.lobby.close_library();
                }
                SettingsPress::OpenSettings.handle(cx);
            }
            HeaderPress::Nav(n) => {
                if cx.state.settings.is_open() {
                    cx.state.settings = SettingsPane::Closed;
                }
                if cx.state.library_open() {
                    cx.state.lobby.close_library();
                }
                let hub = if n == 0 { Hub::Play } else { Hub::Decks };
                HubPress::Tab(hub).handle(cx);
            }
            HeaderPress::Return => back_to_the_table(cx),
            HeaderPress::Leave => {
                if let Some(Strip::Seated { index, .. }) = strips::strip(&cx.state.lobby) {
                    RoomPress::LeaveTable(index).handle(cx);
                }
            }
            HeaderPress::RetryNow => {
                cx.state.retry_feed = true;
                HubPress::Refresh.handle(cx);
            }
        }
    }
}

fn toggle(state: &mut LobbyState, menu: HeaderMenu) {
    state.header_menu = if state.header_menu == Some(menu) {
        None
    } else {
        Some(menu)
    };
}

/// Return: to the waiting room (close what stands over it — Settings, the
/// house decks, the builder, which asks first when there is something
/// unsaved), or back into the game being played.
fn back_to_the_table(cx: Cx<'_, '_, '_, '_, '_>) {
    match strips::strip(&cx.state.lobby) {
        Some(Strip::Seated { .. }) => {
            if cx.state.settings.is_open() {
                cx.state.settings = SettingsPane::Closed;
            }
            if cx.state.library_open() {
                cx.state.lobby.close_library();
            }
            if matches!(cx.state.lobby.screen(), Screen::Build) {
                BuildPress::CloseBuilder.handle(cx);
            }
        }
        Some(Strip::Playing { game_id, .. }) => {
            let request = cx.state.lobby.return_to_game(&game_id);
            dispatch(cx.state, cx.mailbox, request);
        }
        None => {}
    }
}

impl LobbyState {
    /// The chosen gateway's own name, as `GET /info` gave it.
    pub(super) fn gateway_name(&self) -> Option<String> {
        match self.probes.get(&self.gateway) {
            Some(Probe::Known(info)) => info.name.clone().filter(|n| !n.trim().is_empty()),
            _ => None,
        }
    }

    /// The gateway in words, for Settings › Network & Gateway: its name
    /// (or address), the address, its version with the verdict.
    pub(crate) fn gateway_facts(&self, lang: Lang) -> String {
        let words = super::gateway::row_words(&self.gateway, self.probes.get(&self.gateway), lang);
        let verdict = words.warning.map_or_else(
            || Phrase::FrontCompatible.text(lang).to_string(),
            |w| w.explain(lang),
        );
        let mut said = words.title.clone();
        if let Some(address) = &words.address {
            said.push_str(" \u{b7} ");
            said.push_str(address);
        }
        let _ = write!(
            said,
            " \u{b7} {} \u{b7} {verdict} \u{b7} {} {}",
            words.version_in_full,
            Phrase::ShellThisClient.text(lang),
            baylee_build::short()
        );
        said
    }

    /// The connection's state as text, to copy into a report (Settings ›
    /// Network & Gateway's diagnostics): the gateway, its reach, the feed,
    /// the session's kind. No token, no account id.
    pub(crate) fn diagnostics(&self) -> String {
        let reach = match self.probes.get(&self.gateway) {
            Some(Probe::Known(info)) => format!("answering, {}", info.version),
            Some(Probe::Older) => "answering, older than /info".to_string(),
            Some(Probe::Silent) => "not answering".to_string(),
            Some(Probe::Asking) | None => "not asked".to_string(),
        };
        format!(
            "gateway {} ({reach}); feed {}; session {}; client {}",
            if self.gateway.is_empty() {
                "none"
            } else {
                &self.gateway
            },
            if self.feed_down { "down" } else { "up" },
            if self.lobby.offline() {
                "offline"
            } else if self.lobby.guest() {
                "guest"
            } else if self.lobby.token().is_some() {
                "account"
            } else {
                "none"
            },
            baylee_build::short()
        )
    }

    /// Whether the house decks' page stands over the hub.
    pub(super) fn library_open(&self) -> bool {
        self.lobby.library().page.is_some()
    }

    /// Whether the chosen gateway now refuses this client's games: it was
    /// upgraded under a running client (§2.5).
    pub(super) fn update_required(&self) -> bool {
        self.lobby.token().is_some()
            && self.probes.get(&self.gateway).is_some_and(|probe| {
                probe
                    .warning(baylee_protocol::PROTOCOL_VERSION, baylee_view::VIEW_VERSION)
                    .is_some_and(client_core::lobby::gateway_info::Warning::refuses_games)
            })
    }
}

/// How often the gateway pill's counts are asked again.
const STATS_EVERY: f32 = 20.0;
/// How often a signed-in client asks its gateway's `/info` again, to notice
/// an upgrade under it.
const INFO_EVERY: f32 = 300.0;

/// What [`follow_the_account`] has asked, and when it asks next.
#[derive(Default)]
pub(super) struct Asked {
    me_for: Option<String>,
    stats_in: f32,
    info_in: f32,
}

/// Asks `GET /me` once a session, `GET /lobby/stats` every
/// [`STATS_EVERY`] seconds on the hub, and `/info` every [`INFO_EVERY`]:
/// side questions off the lobby's one-request chain, read-only here.
pub(super) fn follow_the_account(
    time: Res<Time>,
    state: Res<LobbyState>,
    mailbox: Res<Mailbox>,
    mut asked: Local<Asked>,
) {
    let token = match state.lobby.token() {
        Some(token) if !state.lobby.offline() && !state.gateway.is_empty() => token.to_string(),
        _ => {
            *asked = Asked::default();
            return;
        }
    };
    if asked.me_for.as_deref() != Some(token.as_str()) {
        // A new session: who it is, at once; the counts at once; `/info`
        // again only after a while (choosing the gateway just asked it).
        if state.lobby.me().is_none() {
            http::ask_aside(
                &state.gateway,
                "/me",
                &token,
                state.gateway_epoch,
                &mailbox,
                Reply::Me,
            );
        }
        *asked = Asked {
            me_for: Some(token.clone()),
            stats_in: 0.0,
            info_in: INFO_EVERY,
        };
    }
    if !matches!(state.lobby.screen(), Screen::Table) {
        return;
    }
    asked.stats_in -= time.delta_secs();
    if asked.stats_in <= 0.0 {
        asked.stats_in = STATS_EVERY;
        http::ask_aside(
            &state.gateway,
            "/lobby/stats",
            &token,
            state.gateway_epoch,
            &mailbox,
            Reply::Stats,
        );
    }
    asked.info_in -= time.delta_secs();
    if asked.info_in <= 0.0 {
        asked.info_in = INFO_EVERY;
        http::probe_gateway(state.gateway.clone(), &mailbox);
    }
}

/// Rings the bell for what changed at the player's waiting table between
/// two listings.
pub(super) fn ring_the_bell(
    mut state: ResMut<LobbyState>,
    mut was: Local<Option<client_core::lobby::GameSummary>>,
) {
    if !state.is_changed() {
        return;
    }
    let now = strips::my_waiting_table(&state.lobby).cloned();
    let news = match (was.as_ref(), now.as_ref()) {
        (Some(before), Some(after)) => strips::table_news(before, after),
        _ => Vec::new(),
    };
    *was = now;
    for item in news {
        state.bell.ring(item);
    }
}

/// The toasts on show: each sentence and the seconds it has left.
#[derive(Resource, Default)]
pub(super) struct Toasts {
    shown: Vec<(String, f32)>,
    rung: u64,
    drawn: Vec<String>,
}

/// How long a toast stays (§2.4: 6 s), and how many show at once (≤ 3).
const TOAST_SECS: f32 = 6.0;
const TOASTS_AT_ONCE: usize = 3;

/// The toast lane: the bell's newest items for six seconds each, at most
/// three, bottom-right (from the top on a Phone). Its own root, redrawn only
/// when what it shows changes.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn toast_lane(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<LobbyState>,
    mut toasts: ResMut<Toasts>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
    input: Res<crate::shellkit::InputClass>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    lanes: Query<Entity, With<ToastLane>>,
) {
    let toasts = &mut *toasts;
    let lang = state.lobby.lang();
    if state.bell.rung() > toasts.rung {
        let new = usize::try_from(state.bell.rung() - toasts.rung).unwrap_or(usize::MAX);
        for item in state.bell.latest(new.min(TOASTS_AT_ONCE)) {
            toasts.shown.push((item.sentence(lang), TOAST_SECS));
        }
        toasts.rung = state.bell.rung();
    }
    let dt = time.delta_secs();
    for (_, left) in &mut toasts.shown {
        *left -= dt;
    }
    toasts.shown.retain(|(_, left)| *left > 0.0);
    while toasts.shown.len() > TOASTS_AT_ONCE {
        toasts.shown.remove(0);
    }
    let now: Vec<String> = toasts.shown.iter().map(|(t, _)| t.clone()).collect();
    if now == toasts.drawn && (now.is_empty() || !lanes.is_empty()) {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    for lane in &lanes {
        commands.entity(lane).despawn();
    }
    toasts.drawn.clone_from(&now);
    if now.is_empty() {
        return;
    }
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let step = settings
        .as_deref()
        .map_or_else(Default::default, |s| s.text_size);
    let kit = Kit {
        fonts: &fonts,
        m: kit_metrics(width, height, step, *input),
        german: lang == Lang::De,
    };
    let phone = kit.m.frame == ShellFrame::Phone;
    let lane = commands
        .spawn((
            ToastLane,
            Node {
                position_type: PositionType::Absolute,
                right: px_fixed(kit.m.body),
                left: if phone {
                    px_fixed(kit.m.body)
                } else {
                    Val::Auto
                },
                top: if phone {
                    px_fixed(kit.m.header + kit.m.gap)
                } else {
                    Val::Auto
                },
                bottom: if phone {
                    Val::Auto
                } else {
                    px_fixed(kit.m.body)
                },
                width: if phone {
                    Val::Auto
                } else {
                    kit.m.px(
                        if kit.m.frame == ShellFrame::Wide || kit.m.frame == ShellFrame::Vast {
                            400.0
                        } else {
                            360.0
                        },
                    )
                },
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            GlobalZIndex(tokens::z::TOAST),
            Pickable::IGNORE,
        ))
        .id();
    for text in &now {
        let toast = surfaces::toast(&mut commands, kit, text, None);
        commands.entity(lane).add_child(toast);
    }
}

/// The toast lane's root.
#[derive(Component)]
pub(crate) struct ToastLane;

/// Carries out what header presses asked of systems they cannot reach: the
/// `?` overlay (through the shell keymap's own message) and the report form.
pub(super) fn carry_out(
    mut state: ResMut<LobbyState>,
    mut fired: MessageWriter<crate::shellkit::keys::ShellFired>,
    desk: Option<ResMut<crate::report::ReportDesk>>,
) {
    if state.shell_asks.is_empty() {
        return;
    }
    let mut desk = desk;
    for ask in std::mem::take(&mut state.shell_asks) {
        match ask {
            ShellAsk::Shortcuts => {
                fired.write(crate::shellkit::keys::ShellFired(
                    baylee_client_core::shellkeys::ShellAction::Overlay,
                ));
            }
            ShellAsk::Report => {
                if let Some(desk) = desk.as_mut() {
                    desk.asked = true;
                }
            }
        }
    }
}

/// The header's systems: its side questions, the bell, the toasts, what its
/// presses ask of others, and the feed's reconnecting flag.
pub(super) fn install(app: &mut App) {
    app.init_resource::<Toasts>().add_systems(
        Update,
        (
            follow_the_account,
            ring_the_bell,
            toast_lane,
            carry_out,
            super::feed::follow_the_feed.after(super::feed::feed),
        )
            .run_if(in_state(DuelPhase::Closed)),
    );
}
