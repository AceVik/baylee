//! The kit's gallery: every component in every state on one dev-only page
//! (the shell design, §17 WP0b-1, M4-5: the package is accepted on what it
//! owns, which is this page).
//!
//! Compiled only with `dev-control`, the harness's own lock, and opened only
//! through it: `POST /gallery {"open":true}`. It stands over the lobby on
//! the painting, as a shell screen will, so the contrast check reads the
//! same pixels a screen's would. Rebuilt only when what it shows changes:
//! the window, the text step, the input class, the language.

use super::ShellMetrics;
use super::controls::{self, Kit, Live, Weight};
use super::metrics::px_fixed;
use super::role::Role;
use super::size::{Frame, InputClass, Platform, TextSize, Viewport};
use super::states;
use super::surfaces::{self, MenuItem, SheetWidth, Storage, TileLook};
use super::tokens;
use crate::hud::{UiFonts, icon_tf, tf, tf_bold};
use crate::settings::ClientSettings;
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Whether the gallery is up.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gallery {
    /// Shown over the lobby.
    pub open: bool,
}

/// The gallery's root; `devctl`'s `shell_nodes` dumps it beside the lobby's.
#[derive(Component)]
pub struct GalleryRoot;

/// The gallery's scrolling body.
#[derive(Component)]
struct GalleryBody;

/// Everything the drawn page depends on.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Drawn {
    width: f32,
    height: f32,
    step: TextSize,
    input: InputClass,
    german: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Gallery>()
        .add_systems(Update, (draw, scroll, hide_the_lobby).chain());
}

/// The lobby's own tree stands aside while the gallery is up: the gallery's
/// panels are translucent, and the contrast check is to read the painting
/// under them, as a shell screen's would, not a lobby button behind them.
fn hide_the_lobby(
    gallery: Res<Gallery>,
    mut roots: Query<(&mut Visibility, Ref<crate::lobby::LobbyRoot>)>,
) {
    for (mut shown, root) in &mut roots {
        if !gallery.is_changed() && !root.is_added() {
            continue;
        }
        let want = if gallery.open {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *shown != want {
            *shown = want;
        }
    }
}

/// Builds the page when it opens or what it depends on changes, and takes
/// it down when it closes.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn draw(
    mut commands: Commands,
    gallery: Res<Gallery>,
    settings: Option<Res<ClientSettings>>,
    input: Res<InputClass>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<GalleryRoot>>,
    mut drawn: Local<Option<Drawn>>,
) {
    if !gallery.open {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *drawn = None;
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    let (width, height) = windows
        .single()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let now = Drawn {
        width,
        height,
        step: settings.as_deref().map_or(TextSize::L, |s| s.text_size),
        input: *input,
        german: lang == Lang::De,
    };
    if *drawn == Some(now) && !roots.is_empty() {
        return;
    }
    *drawn = Some(now);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let view = Viewport {
        width,
        height,
        platform: Platform::current(),
        input: now.input,
    };
    let kit = Kit {
        fonts: &fonts,
        m: ShellMetrics::of(view, now.step),
        german: now.german,
    };
    page(&mut commands, kit, lang);
}

/// Scrolls the body under the wheel; the page is taller than a phone.
fn scroll(
    mut wheels: MessageReader<MouseWheel>,
    mut bodies: Query<(&mut ScrollPosition, &ComputedNode), With<GalleryBody>>,
) {
    let delta: f32 = wheels
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y * 32.0,
            MouseScrollUnit::Pixel => w.y,
        })
        .sum();
    if delta == 0.0 {
        return;
    }
    for (mut at, node) in &mut bodies {
        let room = (node.content_size().y - node.size().y).max(0.0) * node.inverse_scale_factor();
        at.y = (at.y - delta).clamp(0.0, room);
    }
}

fn page(commands: &mut Commands, kit: Kit, lang: Lang) {
    let m = kit.m;
    let root = commands
        .spawn((
            GalleryRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(5),
            Pickable::IGNORE,
        ))
        .id();
    let head = header(commands, kit, lang);
    let body = commands
        .spawn((
            GalleryBody,
            Node {
                flex_grow: 1.0,
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px_fixed(m.body)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .id();
    let wrap = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: px_fixed(m.collection_max),
                flex_wrap: FlexWrap::Wrap,
                column_gap: px_fixed(m.body),
                row_gap: px_fixed(m.body),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let panels = [
        type_panel(commands, kit, lang),
        buttons_panel(commands, kit, lang),
        choices_panel(commands, kit, lang),
        rows_panel(commands, kit, lang),
        surfaces_panel(commands, kit, lang),
        lists_panel(commands, kit, lang),
        tiles_panel(commands, kit, lang),
        sheet_panel(commands, kit, lang),
    ];
    commands.entity(wrap).add_children(&panels);
    commands.entity(body).add_child(wrap);
    commands.entity(root).add_children(&[head, body]);
}

/// A panel of the gallery: grows to fill the row, never under 320 × factor.
fn section(commands: &mut Commands, kit: Kit, title: &str) -> Entity {
    let width = if kit.m.frame == Frame::Phone {
        Val::Percent(100.0)
    } else {
        Val::Auto
    };
    let panel = surfaces::panel(commands, kit, width);
    commands.entity(panel).insert(Node {
        width,
        min_width: kit.m.px(320.0),
        max_width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_basis: kit.m.px(420.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(px_fixed(kit.m.pad)),
        row_gap: px_fixed(kit.m.gap),
        border: UiRect::all(px_fixed(1.0)),
        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
        ..default()
    });
    let title = surfaces::heading(commands, kit, title);
    commands.entity(panel).add_child(title);
    panel
}

/// A row of controls that wraps.
fn line(commands: &mut Commands, kit: Kit, children: &[Entity]) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_children(children);
    line
}

fn header(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let m = kit.m;
    let bar = commands
        .spawn((
            Role::Header,
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(m.header),
                padding: UiRect::axes(px_fixed(m.body), px_fixed(0.0)),
                column_gap: px_fixed(m.gap),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::PANEL),
        ))
        .id();
    let brand = commands
        .spawn((
            Text::new(Phrase::AppName.text(lang)),
            tf_bold(kit.fonts, m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(brand);
    // The build's short form beside the wordmark on Wide and Vast only
    // (`legal.md`); Narrow and Phone keep it in the popover and colophon.
    if matches!(m.frame, Frame::Wide | Frame::Vast) {
        let build = controls::label(commands, kit, "0.1.0-beta.5", m.small, tokens::MUTED);
        commands.entity(bar).add_child(build);
    }
    for (i, phrase) in [Phrase::ShellPlay, Phrase::ShellDecks, Phrase::Settings]
        .into_iter()
        .enumerate()
    {
        let nav = controls::nav(commands, kit, phrase.text(lang), i == 0, ());
        commands.entity(bar).add_child(nav);
    }
    let gap = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(gap);
    let reach = if matches!(m.frame, Frame::Wide | Frame::Vast) {
        "Baylee Sanctuary · 3 tables · 12 online"
    } else {
        "Baylee Sanctuary"
    };
    let gateway = controls::pill(commands, kit, Some(tokens::ACCENT), reach, true, ());
    let bell = commands
        .spawn((
            Text::new("\u{f0f3}"),
            icon_tf(kit.fonts, m.text),
            TextColor(tokens::INK),
        ))
        .id();
    let bell = controls::hit(commands, kit, bell, ());
    let account = controls::pill(commands, kit, None, "AceVik#0007", true, ());
    commands.entity(bar).add_children(&[gateway, bell, account]);
    bar
}

fn type_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let m = kit.m;
    let panel = section(commands, kit, Phrase::ShellGallery.text(lang));
    let h1 = commands
        .spawn((
            Text::new("Weltenbaum"),
            tf(kit.fonts, m.h1),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let body = surfaces::prose(commands, kit, Phrase::ShellTextSizeHelp.text(lang), false);
    let small = surfaces::prose(commands, kit, Phrase::ShellUpdatedJustNow.text(lang), true);
    let plate = surfaces::mist(
        commands,
        kit,
        Phrase::ShellSeatedAt.fill(lang, &["Thursday pod"]).as_str(),
    );
    commands
        .entity(panel)
        .add_children(&[h1, body, small, plate]);
    panel
}

fn buttons_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::CreateTable.text(lang));
    let primary = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("C"),
        (),
    );
    let gold = controls::button(
        commands,
        kit,
        Phrase::ShellReturnToGame.text(lang),
        Weight::Gold,
        Live::Yes,
        Some("R"),
        (),
    );
    let secondary = controls::button(
        commands,
        kit,
        Phrase::ShellEdit.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (),
    );
    let ghost = controls::button(
        commands,
        kit,
        Phrase::ShellDetails.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        (),
    );
    let danger = controls::button(
        commands,
        kit,
        Phrase::Delete.text(lang),
        Weight::Danger,
        Live::Yes,
        None,
        (),
    );
    let dead = controls::button(
        commands,
        kit,
        Phrase::PlayTheHouse.text(lang),
        Weight::Primary,
        Live::No(Phrase::ShellNeedsGateway.text(lang)),
        None,
        (),
    );
    let first = line(commands, kit, &[primary, gold]);
    let second = line(commands, kit, &[secondary, ghost, danger]);
    let third = line(commands, kit, &[dead]);
    commands.entity(panel).add_children(&[first, second, third]);
    panel
}

fn choices_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::ShellOpenSeats.text(lang));
    let on = controls::chip(
        commands,
        kit,
        Phrase::ShellOpenSeats.text(lang),
        true,
        None,
        false,
        (),
    );
    let off = controls::chip(
        commands,
        kit,
        Phrase::ShellNoPassword.text(lang),
        false,
        Some(4),
        false,
        (),
    );
    let gone = controls::chip(commands, kit, "Commander", true, None, true, ());
    let chips = line(commands, kit, &[on, off, gone]);
    let tabs = controls::tabs(
        commands,
        kit,
        &[
            (Phrase::ShellMyDecks.text(lang), Some(4)),
            (Phrase::ShellHouseDecks.text(lang), Some(18)),
        ],
        0,
        |_| (),
    );
    let sizes = controls::segmented(commands, kit, &["A", "A", "A", "A", "A"], 3, |_| ());
    let toggles = {
        let on = controls::toggle(commands, kit, true, ());
        let off = controls::toggle(commands, kit, false, ());
        let stepper = controls::stepper(commands, kit, "4", (), ());
        line(commands, kit, &[on, off, stepper])
    };
    let search_empty = controls::search(commands, kit, "", Phrase::SearchTables.text(lang), ());
    let search_full = controls::search(
        commands,
        kit,
        "thursday",
        Phrase::SearchTables.text(lang),
        (),
    );
    commands
        .entity(panel)
        .add_children(&[chips, tabs, sizes, toggles, search_empty, search_full]);
    panel
}

fn rows_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::Settings.text(lang));
    let size = controls::segmented(commands, kit, &["XS", "S", "M", "L", "XL"], 3, |_| ());
    let size = surfaces::row(
        commands,
        kit,
        Phrase::ShellTextSize.text(lang),
        Some(Phrase::ShellTextSizeHelp.text(lang)),
        size,
        Some((Storage::Device, Phrase::ShellThisDevice.text(lang))),
    );
    let volume = controls::slider(commands, kit, 70, ());
    let holder = commands
        .spawn((
            Node {
                width: kit.m.px(220.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(volume);
    let volume = surfaces::row(
        commands,
        kit,
        Phrase::ShellMasterVolume.text(lang),
        None,
        holder,
        Some((Storage::Device, Phrase::ShellThisDevice.text(lang))),
    );
    let still = controls::toggle(commands, kit, false, ());
    let still = surfaces::row(
        commands,
        kit,
        Phrase::ShellHoldStill.text(lang),
        None,
        still,
        Some((Storage::Account, Phrase::ShellYourAccount.text(lang))),
    );
    let advanced_body = surfaces::prose(commands, kit, Phrase::ShellNeedsGateway.text(lang), true);
    let drawer = surfaces::drawer(
        commands,
        kit,
        Phrase::ShellAdvanced.text(lang),
        true,
        &[advanced_body],
        (),
    );
    commands
        .entity(panel)
        .add_children(&[size, volume, still, drawer]);
    panel
}

fn surfaces_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::ShellDetails.text(lang));
    let menu = surfaces::menu(
        commands,
        kit,
        [
            MenuItem {
                text: Phrase::ShellEdit.text(lang),
                keys: Some("E"),
                destructive: false,
                action: (),
            },
            MenuItem {
                text: Phrase::ShellCopyHandle.text(lang),
                keys: None,
                destructive: false,
                action: (),
            },
            MenuItem {
                text: Phrase::Delete.text(lang),
                keys: Some("Del"),
                destructive: true,
                action: (),
            },
        ],
    );
    let popover = surfaces::popover(
        commands,
        kit,
        &[
            "Baylee Sanctuary",
            "https://baylee.acevik.de",
            "0.1.0-beta.5+build.2446",
        ],
    );
    let pair = line(commands, kit, &[menu, popover]);
    let undo = controls::button(
        commands,
        kit,
        Phrase::ShellUndo.text(lang),
        Weight::Ghost,
        Live::Yes,
        Some("⌘Z"),
        (),
    );
    let toast = surfaces::toast(
        commands,
        kit,
        Phrase::ShellDeckDeleted.text(lang),
        Some(undo),
    );
    commands.entity(panel).add_children(&[pair, toast]);
    panel
}

fn lists_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::ShellPlay.text(lang));
    let join = controls::button(
        commands,
        kit,
        Phrase::Join.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (),
    );
    let first = surfaces::list_row(
        commands,
        kit,
        "Thursday pod",
        "Maik#0012 · Commander · 40",
        &[join],
        (),
    );
    let second = surfaces::list_row(commands, kit, "quick duel", "Guest#0013 · 20", &[], ());
    let skeleton = states::skeleton(commands, kit, 2);
    let retry = controls::button(
        commands,
        kit,
        Phrase::ShellRetry.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (),
    );
    let error = states::error_line(
        commands,
        kit,
        &Phrase::ShellCouldNotReach.fill(lang, &["Baylee Sanctuary"]),
        retry,
    );
    let create = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        (),
    );
    let empty = states::empty(
        commands,
        kit,
        '\u{f0c0}',
        Phrase::ShellNoOneWaiting.text(lang),
        Some(create),
    );
    commands
        .entity(panel)
        .add_children(&[first, second, skeleton, error, empty]);
    panel
}

fn tiles_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::ShellMyDecks.text(lang));
    let grid = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (name, identity, cards) in [("Weltenbaum", "WUG", 100), ("Goblins", "R", 60)] {
        let use_it = controls::button(
            commands,
            kit,
            Phrase::ShellUseForNextGame.text(lang),
            Weight::Primary,
            Live::Yes,
            None,
            (),
        );
        let edit = controls::button(
            commands,
            kit,
            Phrase::ShellEdit.text(lang),
            Weight::Secondary,
            Live::Yes,
            Some("E"),
            (),
        );
        let meta = Phrase::LibraryCounts.fill(lang, &[&cards.to_string(), "0"]);
        let tile = surfaces::tile(
            commands,
            kit,
            &TileLook {
                name,
                identity,
                meta: &meta,
                badges: "Commander",
            },
            &[use_it, edit],
        );
        commands.entity(tile).insert(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: kit.m.px(280.0),
            min_height: px_fixed(kit.m.tile),
            padding: UiRect::all(kit.m.px(12.0)),
            row_gap: kit.m.px(6.0),
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
            overflow: Overflow::clip(),
            ..default()
        });
        commands.entity(grid).add_child(tile);
    }
    let house = controls::button(
        commands,
        kit,
        Phrase::ShellAddAndUse.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (),
    );
    let more = line(commands, kit, &[house]);
    commands.entity(panel).add_children(&[grid, more]);
    panel
}

fn sheet_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::CreateTable.text(lang));
    let field = controls::search(
        commands,
        kit,
        "Thursday pod",
        Phrase::SearchTables.text(lang),
        (),
    );
    let stepper = controls::stepper(commands, kit, "4", (), ());
    let cancel = controls::button(
        commands,
        kit,
        Phrase::ActCancel.text(lang),
        Weight::Ghost,
        Live::Yes,
        Some("Esc"),
        (),
    );
    let open = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("↵"),
        (),
    );
    let sheet = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::CreateTable.text(lang),
        &[field, stepper],
        &[cancel, open],
    );
    commands.entity(sheet).insert(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        border: UiRect::all(px_fixed(1.0)),
        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
        ..default()
    });
    commands.entity(panel).add_child(sheet);
    panel
}
