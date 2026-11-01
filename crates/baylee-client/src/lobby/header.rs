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
    /// Nav pill `n`: Lobby, Decks, Replays.
    Nav(u8),
    /// The gateway pill (or its dot).
    Gateway,
    /// The bell.
    Bell,
    /// The account pill (or its letter).
    Account,
    /// The chat tool.
    Chat,
    /// Click outside an open header popover to close it.
    CloseMenu,
    /// A toast's cross: the toast it stands on.
    DismissToast(ToastId),
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
    /// The friends button: the friends drawer.
    Friends,
    /// Settings → Privacy's Chat & privacy row: the chat's privacy sheet.
    ChatPrivacy,
    /// The account menu: quit / close the client.
    Quit,
}

/// Something a header press asks of a system the press handler cannot
/// reach (the kit's overlay, the report form).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ShellAsk {
    /// Open the `?` overlay.
    Shortcuts,
    /// Open the report form.
    Report,
    /// Open (or hide) the chat panel.
    Chat,
    /// Open or shut the friends drawer.
    Friends,
    /// The chat's privacy sheet.
    ChatPrivacy,
    /// Dismiss a toast in `Toasts`.
    DismissToast(ToastId),
    /// Quit the client application.
    Quit,
}

/// Marks a header dropdown holder so `place_header_popovers` centers it
/// beneath its [`kit_header::HeaderTrigger`] button and aligns its
/// speech-bubble pointer (`nipple`) directly under the button's center.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HeaderPopover {
    pub(crate) trigger: kit_header::HeaderTrigger,
    pub(crate) nipple: Entity,
}

/// Tracks the currently open header popover for smooth open/switch animation.
#[derive(Resource, Default)]
pub(crate) struct PopoverMotion {
    last_trigger: Option<kit_header::HeaderTrigger>,
    progress: f32,
}

/// Spawns a 14×14 rotated diamond speech-bubble pointer (`Sprechblasen-Nippel`)
/// at the top edge of a dropdown popover.
pub(crate) fn spawn_popover_nipple(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(-7.0),
                right: px_fixed(20.0),
                width: px_fixed(14.0),
                height: px_fixed(14.0),
                border: UiRect {
                    left: px_fixed(1.0),
                    top: px_fixed(1.0),
                    right: px_fixed(0.0),
                    bottom: px_fixed(0.0),
                },
                border_radius: BorderRadius::top_left(px_fixed(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.25, 0.34, 0.98)),
            BorderColor::all(tokens::PRIMARY_EDGE.with_alpha(0.55)),
            UiTransform {
                rotation: Rot2::radians(std::f32::consts::FRAC_PI_4),
                ..default()
            },
            ZIndex(2),
            Pickable::IGNORE,
        ))
        .id()
}

/// One of the header's nav pills.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NavTab {
    /// Lobby (formerly Play).
    Play,
    /// Decks.
    Decks,
    /// Replays (desktop only).
    Replays,
}

/// The nav pills this build draws, in order: a desktop's three (Lobby,
/// Decks, Replays), the web's and a phone's two (`replays::SHOWN`).
/// Settings lives solely on the gear icon.
pub(super) const NAV: &[NavTab] = if super::replays::SHOWN {
    &[NavTab::Play, NavTab::Decks, NavTab::Replays]
} else {
    &[NavTab::Play, NavTab::Decks]
};

/// The press of a nav pill.
pub(super) fn nav_press(tab: NavTab) -> Press {
    let at = NAV.iter().position(|t| *t == tab).unwrap_or(0);
    Press::Header(HeaderPress::Nav(u8::try_from(at).unwrap_or(0)))
}

/// Which nav pill is the screen shown.
fn active(state: &LobbyState) -> Option<usize> {
    if state.settings.is_open() {
        return None;
    }
    let tab = match state.lobby.screen() {
        Screen::Table => match state.hub {
            Hub::Play => NavTab::Play,
            Hub::Decks => NavTab::Decks,
            Hub::Replays => NavTab::Replays,
        },
        _ => return None,
    };
    NAV.iter().position(|t| *t == tab)
}

/// The seated strip the lobby calls for, unless the room itself is shown.
pub(super) fn seated(state: &LobbyState) -> Option<Strip> {
    strips::strip(&state.lobby)
        .filter(|_| state.room_away || !strips::in_the_room(&state.lobby, state.settings.is_open()))
}

/// Whether this screen wears the header: every signed-in screen but the
/// builder, which has its own (its breadcrumb), and the front door.
pub(super) fn wears_header(state: &LobbyState) -> bool {
    let signed_in = state.lobby.token().is_some() || state.lobby.offline();
    signed_in && matches!(state.lobby.screen(), Screen::Table)
}

/// The account pill's handle: `GET /me`'s, a kept guest's, else none yet.
pub(crate) fn handle(state: &LobbyState) -> Option<String> {
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
    retained: Option<&crate::shellkit::retain::Kept>,
) {
    let seated = seated(state);
    if wears_header(state) {
        if let Some(kept) = retained {
            kept.attach(commands, root);
        } else {
            bar(commands, root, state, kit, seated.is_some());
        }
    }
    let builder = matches!(state.lobby.screen(), Screen::Build);
    if state.front_menu && (wears_header(state) || builder) {
        let menu = super::front::gear_menu(commands, state, kit.fonts, metrics, metrics.pad);
        commands.entity(root).add_child(menu);
    }
    strips(commands, root, state, kit, seated);
    // The builder wears a header of its own (`buildui::header`) with the
    // same tools, bell and account, and their popovers are these.
    if let Some(menu) = state.header_menu.filter(|_| wears_header(state) || builder) {
        let popover = popover(commands, state, kit, menu);
        commands.entity(root).add_child(popover);
    }
}

/// Stable header identity, independent of page requests and open popovers.
#[derive(Component)]
pub(crate) struct RetainedHeader(pub String);

/// Only visible header facts invalidate it; page requests and menus do not.
pub(super) fn identity(state: &LobbyState, kit: Kit) -> String {
    let lobby = &state.lobby;
    let gateway = state
        .gateway_name()
        .unwrap_or_else(|| state.gateway.clone());
    let counts = if lobby.offline() {
        Phrase::ShellOfflinePill.text(lobby.lang()).to_owned()
    } else {
        strips::pill_words(lobby.stats(), lobby, lobby.lang())
    };
    format!(
        "{:?}|{}|{}|{}|{}|{:?}|{}|{}|{}|{:?}|{}|{}|{}|{}|{}",
        active(state),
        lobby.offline(),
        state.feed_down,
        gateway,
        counts,
        handle(state),
        seated(state).is_some(),
        state.bell.unread(),
        state.chat_header == Some(true),
        kit.m.frame,
        kit.m.scale,
        kit.m.text,
        kit.m.header,
        kit.m.hit,
        kit.german
    )
}

/// Discards a header when the next screen no longer wears one.
fn discard_detached(
    mut commands: Commands,
    headers: Query<Entity, (With<RetainedHeader>, Without<ChildOf>)>,
) {
    for entity in &headers {
        commands.entity(entity).despawn();
    }
}

/// The header bar itself, and the quick settings' menu when it is open.
#[allow(clippy::too_many_lines)]
fn bar(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit, seated: bool) {
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
        let tools = tools(commands, state, kit);
        let look = HeaderLook {
            brand: Phrase::AppName.text(lang),
            // The short form (§2.1); the full one is in the gateway popover.
            build: baylee_build::VERSION,
            nav: &NAV
                .iter()
                .map(|tab| match tab {
                    NavTab::Play => Phrase::ShellPlay.text(lang),
                    NavTab::Decks => Phrase::ShellDecks.text(lang),
                    NavTab::Replays => {
                        baylee_client_core::replay::words::ReplayPhrase::NavReplays.text(lang)
                    }
                })
                .collect::<Vec<_>>(),
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
        // Clicking the Baylee brand logo navigates to Lobby just like clicking
        // the first nav item (`HeaderPress::Nav(0)`).
        commands.queue(move |world: &mut World| {
            let brand = world.get::<Children>(bar).and_then(|c| {
                c.iter()
                    .find(|e| world.get::<kit_header::BrandMark>(*e).is_some())
            });
            if let Some(brand) = brand
                && let Ok(mut ent) = world.get_entity_mut(brand)
            {
                ent.insert(Press::Header(HeaderPress::Nav(0)));
            }
        });
        commands
            .entity(bar)
            .insert((RetainedHeader(identity(state, kit)), LobbyScreen));
        commands.entity(root).add_child(bar);
    }
}

/// The quick settings and social tools shared by the lobby and builder.
pub(crate) fn tools(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Vec<Entity> {
    let gear_face = commands
        .spawn((
            Node {
                min_width: kit.m.px(32.0),
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(6.0), kit.m.px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            Pickable::IGNORE,
        ))
        .id();
    let gear_glyph = commands
        .spawn((
            Text::new("\u{f013}"),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(gear_face).add_child(gear_glyph);
    let gear = crate::shellkit::controls::hit(
        commands,
        kit,
        gear_face,
        (
            Press::Front(FrontPress::FrontMenu),
            kit_header::HeaderTrigger::Gear,
        ),
    );
    let mut tools = vec![gear];
    if let Some(true) = state.chat_header {
        tools.push(chat_tool(
            commands,
            kit,
            crate::hud::glyph::CHAT,
            Some(Phrase::ShellChat.text(state.lobby.lang())),
            crate::social::HeaderCount::Chat,
            Press::Header(HeaderPress::Chat),
        ));
        tools.push(chat_tool(
            commands,
            kit,
            crate::hud::glyph::FRIENDS,
            None,
            crate::social::HeaderCount::Friends,
            Press::Header(HeaderPress::Friends),
        ));
    }
    tools
}

/// A social tool button in the header: its glyph, and for `HeaderCount::Chat`
/// a notification-bell-style badge dot (`1`..`99+`), or for `Friends` the
/// online count beside the icon.
fn chat_tool(
    commands: &mut Commands,
    kit: crate::shellkit::controls::Kit,
    glyph: char,
    word: Option<&str>,
    count: crate::social::HeaderCount,
    action: Press,
) -> Entity {
    let is_chat_badge = count == crate::social::HeaderCount::Chat;
    let face = commands
        .spawn((
            Node {
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(4.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            crate::shellkit::sheen::Sheen,
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    // The glyph stands in a box of its own, which the chat's badge hangs
    // from (`badge`).
    let icon = commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_child(glyph)
        .id();
    commands.entity(face).add_child(icon);
    if let Some(word) = word {
        let label = commands
            .spawn((
                Text::new(word.to_string()),
                crate::hud::tf(kit.fonts, kit.m.text),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(label);
    }
    let initial = crate::social::header_count(count);
    if is_chat_badge {
        badge(commands, kit, icon, count, initial);
    } else {
        let n = commands
            .spawn((
                count,
                Text::new(initial),
                crate::hud::tf_bold(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(n);
    }
    crate::shellkit::controls::hit(commands, kit, face, action)
}

/// The chat's count: a bell's badge on the icon's shoulder, standing out of
/// the row. In the row with no gap and a negative margin it lay over the
/// word's last letter ("Cha①", 29.10.2026).
fn badge(
    commands: &mut Commands,
    kit: crate::shellkit::controls::Kit,
    icon: Entity,
    count: crate::social::HeaderCount,
    initial: String,
) {
    let badge = commands
        .spawn((
            crate::shellkit::role::Role::Count,
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(-7.0),
                left: Val::Percent(70.0),
                min_width: px_fixed(16.0),
                min_height: px_fixed(16.0),
                padding: UiRect::axes(px_fixed(4.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::GOLD),
            if initial.is_empty() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
            Pickable::IGNORE,
        ))
        .id();
    let n = commands
        .spawn((
            count,
            Text::new(initial),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.85),
            TextColor(tokens::OPAQUE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(n);
    commands.entity(icon).add_child(badge);
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
        let back = controls::button_compact(
            commands,
            kit,
            Some('\u{f053}'),
            Phrase::ShellReturn.text(lang),
            Weight::Gold,
            Live::Yes,
            Press::Header(HeaderPress::Return),
        );
        let mut actions = vec![back];
        if strip.can_leave() {
            actions.push(controls::button_compact(
                commands,
                kit,
                Some(crate::hud::glyph::CLOSE),
                Phrase::ShellLeave.text(lang),
                Weight::Danger,
                Live::Yes,
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
}

/// Builds the rich Gateway statistics & server info popover card.
#[allow(clippy::too_many_lines)] // one popover: status badge, 5-cell breakdown grid, address & version rows
fn gateway_popover(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let name = state
        .gateway_name()
        .unwrap_or_else(|| state.gateway.clone());
    let bd = strips::gateway_breakdown(state.lobby.stats(), &state.lobby);
    let card = commands
        .spawn((
            crate::shellkit::role::Role::Opaque,
            Node {
                width: kit.m.px(300.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: kit.m.px(10.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE.with_alpha(0.96)),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                Color::srgba(0.16, 0.25, 0.34, 0.98).into(),
                Color::srgba(0.05, 0.08, 0.12, 0.98).into(),
            ])),
            BorderColor::all(tokens::PRIMARY_EDGE.with_alpha(0.55)),
            surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::POPOVER),
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(name),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let addr = commands
        .spawn((
            Text::new(state.gateway.clone()),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let grid = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let items = [
        (
            Phrase::GatewayOnlinePlayers.text(lang),
            bd.online.to_string(),
            tokens::ACCENT,
        ),
        (
            Phrase::GatewayInLobby.text(lang),
            bd.in_lobby.to_string(),
            tokens::INK,
        ),
        (
            Phrase::GatewayInGames.text(lang),
            bd.playing.to_string(),
            tokens::GOLD,
        ),
        (
            Phrase::GatewayWaitingTables.text(lang),
            bd.tables_waiting.to_string(),
            tokens::INK,
        ),
        (
            Phrase::GatewayRunningGames.text(lang),
            bd.games_running.to_string(),
            tokens::INK,
        ),
    ];
    for (label_text, val_text, val_color) in items {
        let cell = commands
            .spawn((
                Node {
                    min_width: kit.m.px(82.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(kit.m.px(8.0), kit.m.px(6.0)),
                    row_gap: kit.m.px(2.0),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL.with_alpha(0.75)),
                BorderColor::all(tokens::BORDER),
                Pickable::IGNORE,
            ))
            .id();
        let val = commands
            .spawn((
                Text::new(val_text),
                crate::hud::tf_bold(kit.fonts, kit.m.text),
                TextColor(val_color),
                Pickable::IGNORE,
            ))
            .id();
        let cap = commands
            .spawn((
                Text::new(label_text),
                crate::hud::tf(kit.fonts, kit.m.small * 0.9),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(cell).add_children(&[val, cap]);
        commands.entity(grid).add_child(cell);
    }
    let client_line = format!(
        "{} {}",
        Phrase::ShellThisClient.text(lang),
        baylee_build::short()
    );
    let client = commands
        .spawn((
            Text::new(client_line),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(card)
        .add_children(&[title, addr, grid, client]);
    card
}

/// An open header popover, centered under its trigger button with a
/// speech-bubble pointer (`Sprechblasen-Nippel`) and full-screen click-catcher.
#[allow(clippy::too_many_lines)]
fn popover(commands: &mut Commands, state: &LobbyState, kit: Kit, menu: HeaderMenu) -> Entity {
    let lang = state.lobby.lang();
    let trigger = match menu {
        HeaderMenu::Gateway => kit_header::HeaderTrigger::Gateway,
        HeaderMenu::Bell => kit_header::HeaderTrigger::Bell,
        HeaderMenu::Account => kit_header::HeaderTrigger::Account,
    };
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
        HeaderMenu::Gateway => gateway_popover(commands, state, kit),
        HeaderMenu::Bell => {
            let said: Vec<String> = state.bell.lines(lang);
            let lines: Vec<&str> = if said.is_empty() {
                vec![Phrase::ShellBellEmpty.text(lang)]
            } else {
                said.iter().map(String::as_str).collect()
            };
            let pop = surfaces::popover(commands, kit, &lines);
            commands
                .entity(pop)
                .insert(Press::Shared(SharedPress::PickerNothing));
            pop
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
                MenuItem {
                    text: Phrase::ShellQuit.text(lang),
                    keys: None,
                    destructive: true,
                    action: Press::Header(HeaderPress::Quit),
                },
            ],
        ),
    };
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                right: px_fixed(0.0),
                top: px_fixed(0.0),
                bottom: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(-4000.0),
                right: px_fixed(-4000.0),
                top: px_fixed(-4000.0),
                bottom: px_fixed(-4000.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(tokens::z::POPOVER - 2),
            Press::Header(HeaderPress::CloseMenu),
        ))
        .id();
    let nipple = spawn_popover_nipple(commands);
    commands.entity(surface).insert_children(0, &[nipple]);
    let holder = commands
        .spawn((
            HeaderPopover { trigger, nipple },
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(kit.m.header + 8.0),
                right: px_fixed(kit.m.body),
                ..default()
            },
            UiTransform::default(),
            Visibility::Hidden,
            GlobalZIndex(tokens::z::POPOVER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(surface);
    commands.entity(root).add_children(&[veil, holder]);
    root
}

/// Centers any open [`HeaderPopover`] beneath its trigger button in the header,
/// aligns the speech-bubble pointer (`PopoverNipple`) directly under the
/// button's horizontal center, and eases its open animation.
#[allow(clippy::type_complexity)]
pub(super) fn place_header_popovers(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut motion: ResMut<PopoverMotion>,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
    triggers: Query<
        (
            &kit_header::HeaderTrigger,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        Without<HeaderPopover>,
    >,
    mut popovers: Query<(
        &HeaderPopover,
        &ComputedNode,
        &mut Node,
        &mut UiTransform,
        &mut Visibility,
    )>,
    mut nipples: Query<&mut Node, Without<HeaderPopover>>,
) {
    if popovers.is_empty() {
        motion.last_trigger = None;
        motion.progress = 0.0;
        return;
    }
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let win_width = windows.iter().next().map_or(1280.0, |w| {
        crate::hud::scale::desktop_dimensions(w, ui.as_deref()).0
    });
    for (pop, computed, mut node, mut transform, mut shown) in &mut popovers {
        if motion.last_trigger != Some(pop.trigger) {
            motion.last_trigger = Some(pop.trigger);
            motion.progress = if still { 1.0 } else { 0.0 };
        }
        if motion.progress < 1.0 {
            let step = if still {
                1.0
            } else {
                1.0 - (-22.0 * time.delta_secs()).exp()
            };
            motion.progress = (motion.progress + (1.0 - motion.progress) * step).min(1.0);
            if (1.0 - motion.progress) < 0.005 {
                motion.progress = 1.0;
            }
        }
        let t = motion.progress;
        let scale = Vec2::splat(0.985 + 0.015 * t);
        let y_offset = (1.0 - t) * -6.0;
        if transform.scale != scale || transform.translation != Val2::px(0.0, y_offset) {
            transform.scale = scale;
            transform.translation = Val2::px(0.0, y_offset);
        }

        let Some((_, trig_node, trig_global)) =
            triggers.iter().find(|(trig, ..)| **trig == pop.trigger)
        else {
            continue;
        };
        let trig_scale = trig_node.inverse_scale_factor;
        let trig_size = trig_node.size() * trig_scale;
        if trig_size.x <= 0.0 {
            continue;
        }
        let trig_center = trig_global.translation * trig_scale;
        let anchor_x = trig_center.x;
        let anchor_bottom = trig_center.y + trig_size.y * 0.5;
        let measured_width = computed.size().x * computed.inverse_scale_factor;
        if measured_width <= 0.0 {
            continue;
        }
        let pop_width = measured_width;
        let margin = 12.0;
        let max_left = (win_width - pop_width - margin).max(margin);
        let left = (anchor_x - pop_width * 0.5).clamp(margin, max_left);
        let top = anchor_bottom + 8.0;
        if node.left != px_fixed(left) || node.right != Val::Auto || node.top != px_fixed(top) {
            node.left = px_fixed(left);
            node.right = Val::Auto;
            node.top = px_fixed(top);
            // Layout must consume this position before any pixels are shown.
            continue;
        }
        if *shown != Visibility::Inherited {
            *shown = Visibility::Inherited;
        }
        let nipple_left = (anchor_x - left - 7.0).clamp(16.0, (pop_width - 30.0).max(16.0));
        if let Ok(mut nip) = nipples.get_mut(pop.nipple)
            && (nip.left != px_fixed(nipple_left) || nip.right != Val::Auto)
        {
            nip.left = px_fixed(nipple_left);
            nip.right = Val::Auto;
        }
    }
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
            HeaderPress::CloseMenu => {
                cx.state.header_menu = None;
            }
            HeaderPress::DismissToast(ToastId::Undo) => cx.state.undo = None,
            HeaderPress::DismissToast(id) => cx.state.shell_asks.push(ShellAsk::DismissToast(id)),
            HeaderPress::Shortcuts => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Shortcuts);
            }
            HeaderPress::Report => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Report);
            }
            HeaderPress::Friends | HeaderPress::Chat => {
                cx.state.header_menu = None;
                cx.state.front_menu = false;
                cx.state.shell_asks.push(if self == HeaderPress::Chat {
                    ShellAsk::Chat
                } else {
                    ShellAsk::Friends
                });
            }
            HeaderPress::ChatPrivacy => cx.state.shell_asks.push(ShellAsk::ChatPrivacy),
            HeaderPress::SignOut => {
                cx.state.header_menu = None;
                HubPress::SignOut.handle(cx);
            }
            HeaderPress::Quit => {
                cx.state.header_menu = None;
                cx.state.shell_asks.push(ShellAsk::Quit);
            }
            HeaderPress::Nav(n) => {
                cx.state.header_menu = None;
                cx.state.front_menu = false;
                if cx.state.lobby.awaiting().is_some() && !cx.state.room_away {
                    cx.state.room_away = true;
                }
                if cx.state.settings.is_open() {
                    cx.state.settings = SettingsPane::Closed;
                }
                if cx.state.library_open() {
                    cx.state.lobby.close_library();
                }
                let hub = match NAV.get(usize::from(n)) {
                    Some(NavTab::Decks) => Hub::Decks,
                    Some(NavTab::Replays) => Hub::Replays,
                    _ => Hub::Play,
                };
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
    state.front_menu = false;
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
            if cx.state.room_away {
                cx.state.room_away = false;
            }
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
        // A deck's history: the house list is the Decks screen's data (WP3),
        // not a screen of its own.
        matches!(
            self.lobby.library().page,
            Some(client_core::lobby::library::Page::History(_))
        )
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

/// What kind of item stands in `ToastLane`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToastKind {
    /// A lobby status or error notice (`lobby.status()`).
    Status(Tone),
    /// A table notification from the bell.
    Bell,
    /// An undoable action toast.
    Undo,
}

/// Which toast a cross takes away: the Undo, or one of [`Toasts`] by the
/// number it was given when it came. Never by where it stands in the lane: a
/// status arrives at the front, so a place read when the cross was drawn can
/// name its neighbour by the time the press is carried out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ToastId {
    /// The Decks screen's Undo (`LobbyState::undo`).
    Undo,
    /// A status or bell toast.
    Shown(u64),
}

/// The toasts on show: each one's number, kind, sentence, and the seconds it
/// has left.
#[derive(Resource, Default)]
pub(super) struct Toasts {
    shown: Vec<(u64, ToastKind, String, f32)>,
    next: u64,
    rung: u64,
    last_status: String,
    drawn: Vec<(ToastId, ToastKind, String)>,
}

impl Toasts {
    /// Gives a toast its number and stands it at `at`.
    fn stand(&mut self, at: usize, kind: ToastKind, said: String) {
        self.next += 1;
        self.shown.insert(
            at.min(self.shown.len()),
            (self.next, kind, said, TOAST_SECS),
        );
    }
}

/// How long a toast stays (15 s), and how many show at once (≤ 4).
const TOAST_SECS: f32 = 15.0;
const TOASTS_AT_ONCE: usize = 4;

/// The unified toast lane at the top-left: lobby status/error notices and
/// the bell's newest items for 15 seconds each, dismissible via `×`, narrow
/// (290 px) with word-wrapping, gradient background, and `Sheen` animation.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system; one lane, read in order
pub(super) fn toast_lane(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<LobbyState>,
    mut toasts: ResMut<Toasts>,
    fonts: Option<Res<UiFonts>>,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
    input: Res<crate::shellkit::InputClass>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    lanes: Query<Entity, With<ToastLane>>,
) {
    let toasts = &mut *toasts;
    let lang = state.lobby.lang();
    let on_shell = matches!(state.lobby.screen(), Screen::Table | Screen::Build);
    let current_status = if on_shell { state.lobby.status() } else { "" };
    if current_status.is_empty() {
        toasts.last_status.clear();
        toasts
            .shown
            .retain(|(_, kind, _, _)| !matches!(kind, ToastKind::Status(_)));
    } else if current_status != toasts.last_status {
        toasts.last_status = current_status.to_string();
        toasts
            .shown
            .retain(|(_, kind, _, _)| !matches!(kind, ToastKind::Status(_)));
        let said = toasts.last_status.clone();
        toasts.stand(0, ToastKind::Status(state.lobby.tone()), said);
    }
    if state.bell.rung() > toasts.rung {
        let new = usize::try_from(state.bell.rung() - toasts.rung).unwrap_or(usize::MAX);
        for item in state.bell.latest(new.min(TOASTS_AT_ONCE)) {
            toasts.stand(usize::MAX, ToastKind::Bell, item.sentence(lang));
        }
        toasts.rung = state.bell.rung();
    }
    let dt = time.delta_secs();
    for (_, _, _, left) in &mut toasts.shown {
        *left -= dt;
    }
    toasts.shown.retain(|(_, _, _, left)| *left > 0.0);
    while toasts.shown.len() > TOASTS_AT_ONCE {
        toasts.shown.remove(0);
    }
    // The Undo the Decks screen offers (S-10) stands first, with its action.
    let undo = state.undo.map(|u| match u.kind {
        super::decks::UndoKind::Delete => Phrase::ShellDeckDeleted.text(lang).to_string(),
        super::decks::UndoKind::Restore => Phrase::HistoryRestored.text(lang).to_string(),
        super::decks::UndoKind::ToursOff => Phrase::TourOffToast.text(lang).to_string(),
        super::decks::UndoKind::ToursRestarted => Phrase::TourRestartedToast.text(lang).to_string(),
    });
    let now: Vec<(ToastId, ToastKind, String)> = undo
        .iter()
        .map(|t| (ToastId::Undo, ToastKind::Undo, t.clone()))
        .chain(
            toasts
                .shown
                .iter()
                .map(|(n, k, t, _)| (ToastId::Shown(*n), *k, t.clone())),
        )
        .collect();
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
    let (width, height) = windows.iter().next().map_or((1280.0, 800.0), |w| {
        crate::hud::scale::desktop_dimensions(w, ui.as_deref())
    });
    let step = settings
        .as_deref()
        .map_or_else(Default::default, |s| s.text_size);
    let kit = Kit {
        fonts: &fonts,
        m: kit_metrics(width, height, step, *input),
        german: lang == Lang::De,
    };
    let phone = kit.m.frame == Frame::Phone;
    let lane = commands
        .spawn((
            ToastLane,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Auto,
                right: px_fixed(kit.m.body),
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
                width: kit.m.px(290.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            GlobalZIndex(tokens::z::TOAST),
            Pickable::IGNORE,
        ))
        .id();
    for (id, kind, text) in &now {
        let action = (*id == ToastId::Undo).then(|| {
            controls::button_compact(
                &mut commands,
                kit,
                None,
                Phrase::ShellUndo.text(lang),
                Weight::Ghost,
                Live::Yes,
                Press::Decks(super::decks::DecksPress::Undo),
            )
        });
        let (icon, accent, title) = match kind {
            ToastKind::Status(Tone::Refusal) => (
                '\u{f071}',
                tokens::DANGER,
                Phrase::ToastTitleAlert.text(lang),
            ),
            ToastKind::Status(_) | ToastKind::Undo => (
                '\u{f05a}',
                tokens::ACCENT,
                Phrase::ToastTitleInfo.text(lang),
            ),
            ToastKind::Bell => ('\u{f0f3}', tokens::GOLD, Phrase::ToastTitleTable.text(lang)),
        };
        let toast = surfaces::toast_card(
            &mut commands,
            kit,
            icon,
            accent,
            title,
            text,
            Press::Header(HeaderPress::DismissToast(*id)),
            action,
        );
        commands.entity(lane).add_child(toast);
    }
}

/// The toast lane's root.
#[derive(Component)]
pub(crate) struct ToastLane;

/// Takes the lobby's toasts away as a table opens (owner, 22.10.2026: "they
/// don't go away, and their X doesn't close them").
///
/// The lane is a root of its own, not part of the lobby's screen, so the
/// lobby's teardown leaves it standing; and [`toast_lane`], which ages it,
/// and [`carry_out`], which answers its cross, both run only while the lobby
/// does. A toast that stood when the table opened stayed over the whole game,
/// as old as it was and with a cross that answered nothing. What it said is
/// the lobby's news, stale by the time the player is back, so it does not
/// come back either: the last status stays remembered, and only a new one is
/// news again.
fn put_the_toasts_away(
    mut commands: Commands,
    mut toasts: ResMut<Toasts>,
    lanes: Query<Entity, With<ToastLane>>,
) {
    for lane in &lanes {
        commands.entity(lane).despawn();
    }
    toasts.shown.clear();
    toasts.drawn.clear();
}

/// Carries out what header presses asked of systems they cannot reach: the
/// `?` overlay (through the shell keymap's own message), the report form,
/// and toast dismissal.
pub(super) fn carry_out(
    mut state: ResMut<LobbyState>,
    mut toasts: ResMut<Toasts>,
    mut fired: MessageWriter<crate::shellkit::keys::ShellFired>,
    mut exit: MessageWriter<bevy::app::AppExit>,
    desk: Option<ResMut<crate::report::ReportDesk>>,
    mut chat: Option<ResMut<crate::social::ChatUi>>,
    mut host: Option<ResMut<crate::social::SocialHost>>,
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
            ShellAsk::Quit => {
                exit.write(bevy::app::AppExit::Success);
            }
            ShellAsk::DismissToast(ToastId::Shown(gone)) => {
                toasts.shown.retain(|(n, _, _, _)| *n != gone);
            }
            ShellAsk::DismissToast(ToastId::Undo) => state.undo = None,
            ShellAsk::Chat | ShellAsk::Friends | ShellAsk::ChatPrivacy => {
                if let (Some(ui), Some(host)) = (chat.as_deref_mut(), host.as_deref_mut()) {
                    crate::social::header_ask(host, ui, ask);
                }
            }
        }
    }
}

/// The header's systems: its side questions, the bell, the toasts, what its
/// presses ask of others, and the feed's reconnecting flag.
pub(super) fn install(app: &mut App) {
    app.init_resource::<Toasts>()
        .init_resource::<PopoverMotion>()
        .add_systems(
            Update,
            (
                (
                    follow_the_account,
                    ring_the_bell,
                    toast_lane,
                    super::feed::follow_the_feed.after(super::feed::feed),
                )
                    .run_if(in_state(DuelPhase::Closed)),
                // Over a table too: the settings' Report a problem and Chat
                // privacy rows ask through it (`table_settings`); the header's
                // other asks have no control drawn there.
                carry_out.run_if(super::table_settings::lobby_or_over_table),
            ),
        )
        .add_systems(OnExit(DuelPhase::Closed), put_the_toasts_away)
        .add_systems(
            PostUpdate,
            (discard_detached, place_header_popovers)
                .after(bevy::ui::UiSystems::Layout)
                .run_if(in_state(DuelPhase::Closed)),
        );
}
