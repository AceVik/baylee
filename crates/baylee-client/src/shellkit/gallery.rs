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
use super::focus::{Current, ShellField, Stop, TabOrder};
use super::metrics::px_fixed;

use super::size::{Frame, InputClass, Platform, TextSize, Viewport};
use super::states;
use super::surfaces::{self, MenuItem, SheetWidth, Storage, TileLook};
use super::tokens;
use crate::hud::{UiFonts, tf};
use crate::settings::ClientSettings;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::shellkeys::ShellAction as A;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The gallery's focus order (`KEYBOARD.md` §1.3, §9.5): every control it
/// draws, in reading order. A composite (the nav, the chips, the tabs, a
/// radio group, a stepper, the menu) is one stop walked with the arrows.
pub const GALLERY_ORDER: TabOrder = TabOrder {
    name: "gallery",
    stops: &[
        "nav",
        "gateway",
        "bell",
        "account",
        "create",
        "return",
        "edit",
        "details",
        "delete",
        "play-house",
        "chips",
        "tabs",
        "sizes",
        "toggle-on",
        "toggle-off",
        "stepper",
        "search",
        "search-full",
        "text-size",
        "volume",
        "hold-still",
        "advanced",
        "menu",
        "undo",
        "row-1",
        "join",
        "row-2",
        "retry",
        "create-empty",
        "tile-1-use",
        "tile-1-edit",
        "tile-2-use",
        "tile-2-edit",
        "add-and-use",
        "sheet-name",
        "sheet-players",
        "sheet-cancel",
        "sheet-open",
    ],
    modal: false,
};

const fn stop(id: &'static str) -> Stop {
    Stop::new(GALLERY_ORDER.name, id)
}

const fn item(id: &'static str, n: u8) -> Stop {
    Stop::item(GALLERY_ORDER.name, id, n)
}

/// Whether an item holds its composite's place.
const fn current(here: bool) -> Current {
    Current(here)
}

/// The key caps the gallery shows for the actions it draws, read from the
/// account's shell keymap and what the session has learnt: the page redraws
/// when one changes (§9.3: every drawn cap shows the current key).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct Caps {
    create: Option<String>,
    back_to_game: Option<String>,
    edit: Option<String>,
    undo: Option<String>,
}

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
#[derive(Clone, PartialEq, Debug)]
struct Drawn {
    width: f32,
    height: f32,
    step: TextSize,
    input: InputClass,
    german: bool,
    caps: Caps,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Gallery>()
        .add_systems(Update, (draw, scroll).chain());
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    app.add_systems(Update, hide_the_lobby);
}

/// The lobby's own tree stands aside while the gallery is up: the gallery's
/// panels are translucent, and the contrast check is to read the painting
/// under them, as a shell screen's would, not a lobby button behind them.
#[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
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
    prefs: Option<Res<crate::prefs::Prefs>>,
    learnt: Res<super::keys::LearntKeys>,
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
    let standard = baylee_client_core::shellkeys::ShellKeymap::standard();
    let keymap = prefs.as_deref().map_or(&standard, |p| &p.all().shell_keys);
    let hint = |action| keymap.hint(action, super::keys::mac(), &learnt.0);
    let now = Drawn {
        width,
        height,
        step: settings.as_deref().map_or(TextSize::L, |s| s.text_size),
        input: *input,
        german: lang == Lang::De,
        caps: Caps {
            create: hint(A::CreateTable),
            back_to_game: hint(A::ReturnToGame),
            edit: hint(A::EditTile),
            undo: hint(A::Undo),
        },
    };
    if drawn.as_ref() == Some(&now) && !roots.is_empty() {
        return;
    }
    let caps = now.caps.clone();
    let step = now.step;
    let touch = now.input;
    *drawn = Some(now);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let view = Viewport {
        width,
        height,
        platform: Platform::current(),
        input: touch,
    };
    let kit = Kit {
        fonts: &fonts,
        m: ShellMetrics::of(view, step),
        german: lang == Lang::De,
    };
    page(&mut commands, kit, lang, &caps);
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

fn page(commands: &mut Commands, kit: Kit, lang: Lang, caps: &Caps) {
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
        buttons_panel(commands, kit, lang, caps),
        choices_panel(commands, kit, lang),
        rows_panel(commands, kit, lang),
        surfaces_panel(commands, kit, lang, caps),
        lists_panel(commands, kit, lang),
        tiles_panel(commands, kit, lang, caps),
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

/// The kit's header, as every signed-in screen wears it (§2.1), with the
/// gallery's stops on its controls.
fn header(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let counts = Phrase::ShellTablesOnline.fill(lang, &["3", "12"]);
    let look = super::header::HeaderLook {
        brand: Phrase::AppName.text(lang),
        build: "0.1.0-beta.5",
        nav: [
            Phrase::ShellPlay.text(lang),
            Phrase::ShellDecks.text(lang),
            Phrase::Settings.text(lang),
        ],
        active: Some(0),
        reach: super::header::Reach::Up,
        gateway: "Baylee Sanctuary",
        counts: &counts,
        unread: 1,
        handle: Some("AceVik#0007"),
        return_pill: None,
        tools: &[],
    };
    super::header::header(
        commands,
        kit,
        &look,
        super::header::HeaderActions {
            nav: |i| {
                let n = u8::try_from(i).unwrap_or(0);
                (item("nav", n), current(i == 0))
            },
            gateway: stop("gateway"),
            bell: stop("bell"),
            account: stop("account"),
            back: (),
        },
    )
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

fn buttons_panel(commands: &mut Commands, kit: Kit, lang: Lang, caps: &Caps) -> Entity {
    let panel = section(commands, kit, Phrase::CreateTable.text(lang));
    let primary = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        Live::Yes,
        caps.create.as_deref(),
        stop("create"),
    );
    let gold = controls::button(
        commands,
        kit,
        Phrase::ShellReturnToGame.text(lang),
        Weight::Gold,
        Live::Yes,
        caps.back_to_game.as_deref(),
        stop("return"),
    );
    let secondary = controls::button(
        commands,
        kit,
        Phrase::ShellEdit.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        stop("edit"),
    );
    let ghost = controls::button(
        commands,
        kit,
        Phrase::ShellDetails.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        stop("details"),
    );
    let danger = controls::button(
        commands,
        kit,
        Phrase::Delete.text(lang),
        Weight::Danger,
        Live::Yes,
        None,
        stop("delete"),
    );
    let dead = controls::button(
        commands,
        kit,
        Phrase::PlayTheHouse.text(lang),
        Weight::Primary,
        Live::No(Phrase::ShellNeedsGateway.text(lang)),
        None,
        stop("play-house"),
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
        item("chips", 0),
    );
    let off = controls::chip(
        commands,
        kit,
        Phrase::ShellNoPassword.text(lang),
        false,
        Some(4),
        false,
        item("chips", 1),
    );
    let gone = controls::chip(
        commands,
        kit,
        "Commander",
        true,
        None,
        true,
        item("chips", 2),
    );
    let chips = line(commands, kit, &[on, off, gone]);
    let tabs = controls::tabs(
        commands,
        kit,
        &[
            (Phrase::ShellMyDecks.text(lang), Some(4)),
            (Phrase::ShellHouseDecks.text(lang), Some(18)),
        ],
        0,
        |i| (item("tabs", u8::try_from(i).unwrap_or(0)), current(i == 0)),
    );
    let sizes = controls::segmented(commands, kit, &["A", "A", "A", "A", "A"], 3, |i| {
        (item("sizes", u8::try_from(i).unwrap_or(0)), current(i == 3))
    });
    let toggles = {
        let on = controls::toggle(commands, kit, true, stop("toggle-on"));
        let off = controls::toggle(commands, kit, false, stop("toggle-off"));
        let stepper = controls::stepper(commands, kit, "4", item("stepper", 0), item("stepper", 1));
        line(commands, kit, &[on, off, stepper])
    };
    let hint = Phrase::SearchTables.text(lang);
    let search_empty = controls::search(
        commands,
        kit,
        "",
        hint,
        (stop("search"), ShellField::new("", hint)),
    );
    let search_full = controls::search(
        commands,
        kit,
        "thursday",
        hint,
        (stop("search-full"), ShellField::new("thursday", hint)),
    );
    commands
        .entity(panel)
        .add_children(&[chips, tabs, sizes, toggles, search_empty, search_full]);
    panel
}

fn rows_panel(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let panel = section(commands, kit, Phrase::Settings.text(lang));
    let size = controls::segmented(commands, kit, &["XS", "S", "M", "L", "XL"], 3, |i| {
        (
            item("text-size", u8::try_from(i).unwrap_or(0)),
            current(i == 3),
        )
    });
    let size = surfaces::row(
        commands,
        kit,
        Phrase::ShellTextSize.text(lang),
        Some(Phrase::ShellTextSizeHelp.text(lang)),
        size,
        Some((Storage::Device, Phrase::ShellThisDevice.text(lang))),
    );
    let volume = controls::slider(commands, kit, 70, stop("volume"));
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
    let still = controls::toggle(commands, kit, false, stop("hold-still"));
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
        stop("advanced"),
    );
    commands
        .entity(panel)
        .add_children(&[size, volume, still, drawer]);
    panel
}

fn surfaces_panel(commands: &mut Commands, kit: Kit, lang: Lang, caps: &Caps) -> Entity {
    let panel = section(commands, kit, Phrase::ShellDetails.text(lang));
    let menu = surfaces::menu(
        commands,
        kit,
        [
            MenuItem {
                text: Phrase::ShellEdit.text(lang),
                keys: caps.edit.as_deref(),
                destructive: false,
                action: item("menu", 0),
            },
            MenuItem {
                text: Phrase::ShellCopyHandle.text(lang),
                keys: None,
                destructive: false,
                action: item("menu", 1),
            },
            MenuItem {
                text: Phrase::Delete.text(lang),
                keys: Some("Del"),
                destructive: true,
                action: item("menu", 2),
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
        caps.undo.as_deref(),
        stop("undo"),
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
        stop("join"),
    );
    let first = surfaces::list_row(
        commands,
        kit,
        "Thursday pod",
        "Maik#0012 · Commander · 40",
        &[join],
        stop("row-1"),
    );
    let second = surfaces::list_row(
        commands,
        kit,
        "quick duel",
        "Guest#0013 · 20",
        &[],
        stop("row-2"),
    );
    let skeleton = states::skeleton(commands, kit, 2);
    let retry = controls::button(
        commands,
        kit,
        Phrase::ShellRetry.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        stop("retry"),
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
        stop("create-empty"),
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

fn tiles_panel(commands: &mut Commands, kit: Kit, lang: Lang, caps: &Caps) -> Entity {
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
    for (name, identity, cards, (use_id, edit_id)) in [
        ("Weltenbaum", "WUG", 100, ("tile-1-use", "tile-1-edit")),
        ("Goblins", "R", 60, ("tile-2-use", "tile-2-edit")),
    ] {
        let use_it = controls::button(
            commands,
            kit,
            Phrase::ShellUseForNextGame.text(lang),
            Weight::Primary,
            Live::Yes,
            None,
            stop(use_id),
        );
        let edit = controls::button(
            commands,
            kit,
            Phrase::ShellEdit.text(lang),
            Weight::Secondary,
            Live::Yes,
            caps.edit.as_deref(),
            stop(edit_id),
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
        stop("add-and-use"),
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
        (
            stop("sheet-name"),
            ShellField::new("Thursday pod", Phrase::SearchTables.text(lang)),
        ),
    );
    let stepper = controls::stepper(
        commands,
        kit,
        "4",
        item("sheet-players", 0),
        item("sheet-players", 1),
    );
    let cancel = controls::button(
        commands,
        kit,
        Phrase::ActCancel.text(lang),
        Weight::Ghost,
        Live::Yes,
        Some("Esc"),
        stop("sheet-cancel"),
    );
    let open = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("Enter"),
        stop("sheet-open"),
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
