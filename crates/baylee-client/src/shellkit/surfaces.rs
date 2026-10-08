//! The shell's surfaces (the shell design, §2.4): panels, mist plates,
//! sheets, popovers, menus, drawers, toasts, rows and tiles.
//!
//! Panels are translucent over the painting; sheets, menus and popovers are
//! opaque (§2.2). The ornament — the dock's leather and inlays — stays on the
//! outer panels only (N4-3), which is the screen's business, not the kit's.

use super::controls::{Kit, label};
use super::metrics::px_fixed;
use super::role::Role;
use super::tokens::{self, RADIUS_CONTROL, RADIUS_PANEL, RADIUS_PILL};
use crate::hud::{icon_tf, tf, tf_bold};
use bevy::prelude::*;

/// A translucent panel on the painting.
pub fn panel(commands: &mut Commands, kit: Kit, width: Val) -> Entity {
    commands
        .spawn((
            Role::Panel,
            Node {
                width,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id()
}

/// A mist plate under a line that would otherwise stand on the painting;
/// the line on it is `INK`, never muted (§2.2).
pub fn mist(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    let plate = commands
        .spawn((
            Role::Mist,
            Node {
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(4.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
        ))
        .id();
    let words = label(commands, kit, text, kit.m.small, tokens::INK);
    commands.entity(plate).add_child(words);
    plate
}

/// A heading inside a panel.
pub fn heading(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id()
}

/// A paragraph: wraps, muted or not.
pub fn prose(commands: &mut Commands, kit: Kit, text: &str, muted: bool) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, if muted { kit.m.small } else { kit.m.text }),
            TextColor(if muted { tokens::MUTED } else { tokens::INK }),
            Pickable::IGNORE,
        ))
        .id()
}

/// How wide a sheet is (§2.4): 560, 720 or 960 × factor; a phone's is the
/// whole screen between the side insets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SheetWidth {
    /// 560: a form of a few fields.
    Small,
    /// 720: the terms, a longer form.
    Medium,
    /// 960: history, a list with a detail beside it.
    Large,
}

/// A sheet's box: a head, a body that scrolls, and a footer that stays —
/// sticky above the gesture and keyboard insets on a phone, so the primary
/// is visible without scrolling (C3-9). Opaque. [`sheet`] stands it over a
/// scrim; the gallery shows the box alone.
pub fn sheet_box(
    commands: &mut Commands,
    kit: Kit,
    width: SheetWidth,
    title: &str,
    body: &[Entity],
    footer: &[Entity],
) -> Entity {
    let phone = kit.m.frame == super::Frame::Phone;
    let wide = match width {
        SheetWidth::Small => 560.0,
        SheetWidth::Medium => 720.0,
        SheetWidth::Large => 960.0,
    };
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: if phone {
                    Val::Percent(100.0)
                } else {
                    kit.m.px(wide)
                },
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(if phone { 0.0 } else { RADIUS_PANEL })),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(kit.m.pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = heading(commands, kit, title);
    commands.entity(head).add_child(title);
    let scroller = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), px_fixed(0.0)),
                row_gap: px_fixed(kit.m.gap),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .id();
    commands.entity(scroller).add_children(body);
    let foot = commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(kit.m.pad)),
                column_gap: px_fixed(kit.m.gap),
                justify_content: JustifyContent::FlexEnd,
                flex_shrink: 0.0,
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(foot).add_children(footer);
    commands
        .entity(surface)
        .add_children(&[head, scroller, foot]);
    surface
}

/// A sheet over a scrim, in the sheet band, centred (a phone's fills the
/// screen). Focus goes into it and back to the opener (WP0b-2).
pub fn sheet(commands: &mut Commands, surface: Entity) -> Entity {
    let scrim = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(tokens::SCRIM),
            GlobalZIndex(tokens::z::SHEET),
        ))
        .id();
    commands.entity(scrim).add_child(surface);
    scrim
}

/// A popover: a read-only summary in an opaque box.
pub fn popover(commands: &mut Commands, kit: Kit, lines: &[&str]) -> Entity {
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: kit.m.px(4.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            GlobalZIndex(tokens::z::POPOVER),
        ))
        .id();
    for (i, line) in lines.iter().enumerate() {
        let words = commands
            .spawn((
                Text::new(*line),
                tf(kit.fonts, kit.m.small),
                TextColor(if i == 0 { tokens::INK } else { tokens::MUTED }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(surface).add_child(words);
    }
    surface
}

/// One item of a menu.
pub struct MenuItem<'a, B: Bundle> {
    /// What it says.
    pub text: &'a str,
    /// Its key, drawn under a pointer.
    pub keys: Option<&'a str>,
    /// Whether it destroys something: last, after a rule, in danger ink.
    pub destructive: bool,
    /// What a press does.
    pub action: B,
}

/// A menu from a `⋯` or a caret: items in order, the destructive one last
/// after a rule. Placement (measured, flipped at the window edge, N-3) is
/// the opener's.
pub fn menu<'a, B: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    items: impl IntoIterator<Item = MenuItem<'a, B>>,
) -> Entity {
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                flex_direction: FlexDirection::Column,
                min_width: kit.m.px(200.0),
                padding: UiRect::axes(px_fixed(0.0), kit.m.px(4.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            GlobalZIndex(tokens::z::POPOVER),
        ))
        .id();
    let mut rule_drawn = false;
    for item in items {
        if item.destructive && !rule_drawn {
            rule_drawn = true;
            let rule = commands
                .spawn((
                    Node {
                        height: px_fixed(1.0),
                        margin: UiRect::axes(px_fixed(0.0), kit.m.px(4.0)),
                        ..default()
                    },
                    BackgroundColor(tokens::BORDER),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(surface).add_child(rule);
        }
        let row = commands
            .spawn((
                Role::MenuItem,
                Node {
                    min_height: px_fixed(kit.m.hit),
                    padding: UiRect::axes(kit.m.px(14.0), px_fixed(0.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: kit.m.px(16.0),
                    ..default()
                },
                item.action,
            ))
            .id();
        let ink = if item.destructive {
            tokens::DANGER
        } else {
            tokens::INK
        };
        let words = label(commands, kit, item.text, kit.m.text, ink);
        commands.entity(row).add_child(words);
        if let Some(keys) = item.keys
            && let Some(cap) = super::controls::key_cap(commands, kit, keys)
        {
            commands.entity(row).add_child(cap);
        }
        commands.entity(surface).add_child(row);
    }
    surface
}

/// A drawer: expert fields folded under a labelled line, closed by default.
pub fn drawer(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    open: bool,
    body: &[Entity],
    action: impl Bundle,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Role::Row,
            Node {
                min_height: px_fixed(kit.m.hit),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            action,
        ))
        .id();
    let chevron = commands
        .spawn((
            Text::new(if open { "\u{f078}" } else { "\u{f054}" }),
            icon_tf(kit.fonts, kit.m.small * 0.8),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let words = label(commands, kit, text, kit.m.text, tokens::INK);
    commands.entity(head).add_children(&[chevron, words]);
    commands.entity(column).add_child(head);
    if open {
        commands.entity(column).add_children(body);
    } else {
        for entity in body {
            commands.entity(*entity).despawn();
        }
    }
    column
}

/// A toast: a sentence and, optionally, one action (Undo, Details).
pub fn toast(commands: &mut Commands, kit: Kit, text: &str, action: Option<Entity>) -> Entity {
    let surface = commands
        .spawn((
            Role::Toast,
            Node {
                width: kit.m.px(400.0),
                max_width: Val::Percent(100.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(10.0)),
                column_gap: px_fixed(kit.m.gap),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Node {
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(surface).add_child(words);
    if let Some(action) = action {
        commands.entity(surface).add_child(action);
    }
    surface
}

/// Where a setting is kept (§12): the row says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Storage {
    /// This device's (`ClientSettings`).
    Device,
    /// The account's, on every device (`Preferences`).
    Account,
}

/// A settings row: label, help under it, the control, the storage tag.
pub fn row(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    help: Option<&str>,
    control: Entity,
    tag: Option<(Storage, &str)>,
) -> Entity {
    let line = commands
        .spawn((
            Role::Row,
            Node {
                min_height: px_fixed(kit.m.row.max(kit.m.hit)),
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                // A control wider than the room beside the words (five
                // 44-px segments under a finger on a phone) goes under them
                // rather than out of the row.
                flex_wrap: FlexWrap::Wrap,
                row_gap: px_fixed(kit.m.gap),
                // A row in a scrolling column keeps its height: shrunk, a
                // long help text ran into the next row (beta.6 QA, D6).
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: kit.m.px(160.0),
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = prose(commands, kit, text, false);
    commands.entity(words).add_child(name);
    if let Some(help) = help {
        let help = prose(commands, kit, help, true);
        commands.entity(words).add_child(help);
    }
    commands.entity(line).add_child(words);
    if let Some((storage, said)) = tag {
        let chip = commands
            .spawn((
                Role::Tag,
                Node {
                    padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BorderColor::all(match storage {
                    Storage::Device => tokens::BORDER,
                    Storage::Account => tokens::ACCENT,
                }),
                Pickable::IGNORE,
            ))
            .id();
        let words = label(commands, kit, said, kit.m.small, tokens::MUTED);
        commands.entity(chip).add_child(words);
        commands.entity(line).add_child(chip);
    }
    commands.entity(line).add_child(control);
    line
}

/// A list row: title, meta under it, trailing controls; one Tab stop.
pub fn list_row(
    commands: &mut Commands,
    kit: Kit,
    title: &str,
    meta: &str,
    trailing: &[Entity],
    action: impl Bundle,
) -> Entity {
    let line = commands
        .spawn((
            Role::Row,
            Node {
                min_height: px_fixed(kit.m.row),
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                column_gap: px_fixed(kit.m.gap),
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            action,
        ))
        .id();
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(title),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let meta = label(commands, kit, meta, kit.m.small, tokens::MUTED);
    commands.entity(words).add_children(&[name, meta]);
    commands.entity(line).add_child(words);
    commands.entity(line).add_children(trailing);
    line
}

/// What a deck tile shows.
pub struct TileLook<'a> {
    /// The deck's name.
    pub name: &'a str,
    /// The colour identity, as letters: `"WUG"`.
    pub identity: &'a str,
    /// The meta line: cards, sideboard, when saved.
    pub meta: &'a str,
    /// The badge line (never on the meta line): format, playable share.
    pub badges: &'a str,
}

/// The ground of a colour's disc and the ink of its letter.
fn identity_colours(letter: char) -> (Color, Color) {
    let dark = Color::srgb(0.06, 0.05, 0.04);
    match letter {
        'W' => (Color::srgb(0.95, 0.92, 0.80), dark),
        'U' => (Color::srgb(0.45, 0.66, 0.90), dark),
        'B' => (Color::srgb(0.42, 0.38, 0.40), tokens::INK),
        'R' => (Color::srgb(0.90, 0.48, 0.38), dark),
        'G' => (Color::srgb(0.45, 0.72, 0.48), dark),
        _ => (Color::srgb(0.66, 0.64, 0.60), dark),
    }
}

/// The colour identity as discs, the colour's letter in each (S4-14): colour
/// is never the only carrier. Discs stay 16 px at least, where a letter
/// still reads.
fn identity_discs(commands: &mut Commands, kit: Kit, identity: &str) -> Entity {
    let discs = commands
        .spawn((
            Node {
                column_gap: kit.m.px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let side = 16.0_f32.max(kit.m.scaled(16.0));
    for letter in identity.chars() {
        let (ground, ink) = identity_colours(letter);
        let disc = commands
            .spawn((
                Node {
                    width: px_fixed(side),
                    height: px_fixed(side),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(ground),
                Pickable::IGNORE,
            ))
            .id();
        let mark = commands
            .spawn((
                Text::new(letter.to_string()),
                tf_bold(kit.fonts, 9.0_f32.max(kit.m.scaled(9.0))),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(disc).add_child(mark);
        commands.entity(discs).add_child(disc);
    }
    discs
}

/// A deck tile (§2.4): the art band — here only the identity gradient, since
/// art without a known artist is never shown (C3-2; the `art_crop` loader
/// and credit are WP3's) — name, identity discs with the colour's letter in
/// each (S4-14; on a phone the identity stands as text in the badge line),
/// the meta line, the badge line, and the actions, always visible under
/// Touch.
pub fn tile(commands: &mut Commands, kit: Kit, look: &TileLook, actions: &[Entity]) -> Entity {
    let phone = kit.m.frame == super::Frame::Phone;
    let card = commands
        .spawn((
            Role::Tile,
            Node {
                flex_direction: FlexDirection::Column,
                min_height: px_fixed(kit.m.tile),
                padding: UiRect::all(kit.m.px(12.0)),
                row_gap: kit.m.px(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(RADIUS_PANEL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let top = commands
        .spawn((
            Node {
                column_gap: kit.m.px(12.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // The art band: the identity's first colour fading to its last.
    let first = look.identity.chars().next().unwrap_or('C');
    let last = look.identity.chars().last().unwrap_or('C');
    let band = commands
        .spawn((
            Node {
                width: kit.m.px(82.0),
                height: kit.m.px(60.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(RADIUS_CONTROL)),
                ..default()
            },
            BackgroundGradient::from(LinearGradient::to_right(vec![
                identity_colours(first).0.with_alpha(0.55).into(),
                identity_colours(last).0.with_alpha(0.25).into(),
            ])),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(look.name),
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(words).add_child(name);
    if !phone {
        let discs = identity_discs(commands, kit, look.identity);
        commands.entity(words).add_child(discs);
    }
    commands.entity(top).add_children(&[band, words]);
    let meta = prose(commands, kit, look.meta, true);
    let badges = if phone && !look.identity.is_empty() {
        format!("{} · {}", look.identity, look.badges)
    } else {
        look.badges.to_string()
    };
    let badge_line = prose(commands, kit, &badges, true);
    let row_of_actions = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                margin: UiRect::top(Val::Auto),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row_of_actions).add_children(actions);
    commands
        .entity(card)
        .add_children(&[top, meta, badge_line, row_of_actions]);
    card
}
