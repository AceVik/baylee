//! The bell's list (owner, 25.11.2026: notifications "schöner und besser
//! durchdacht"): a head, then what waits for the player while it is true
//! (unread chat, friend requests, invitations), then what happened at their
//! table, each line with its mark and, for the table, how long ago; the
//! lines new since the bell was last opened are lit. Nothing in it is an
//! empty state that says what will appear, never a bare "nothing new".

#[allow(clippy::wildcard_imports)] // the header's own vocabulary
use super::*;
use crate::hud::glyph;
use client_core::lobby::strips::{BellItem, ago};

/// A line social keeps standing in the bell while it is true, and its mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notice {
    /// The icon-font glyph it stands behind.
    pub(crate) glyph: char,
    /// What it says.
    pub(crate) text: String,
}

impl Notice {
    /// A notice behind `glyph`.
    pub(crate) const fn new(glyph: char, text: String) -> Self {
        Self { glyph, text }
    }
}

/// The list's width, as the gateway's card (300) and the account menu stand.
const WIDTH: f32 = 340.0;
/// How tall the list grows before it scrolls.
const MAX_LIST: f32 = 380.0;

/// The mark a table line stands behind.
const fn mark(item: &BellItem) -> (char, Color) {
    match item {
        BellItem::Joined(_) => (glyph::USER_PLUS, tokens::GAIN),
        BellItem::Left(_) => (glyph::USER_MINUS, tokens::MUTED),
        BellItem::Filled => (glyph::CHECK, tokens::GOLD),
    }
}

/// The bell's popover.
pub(super) fn popover(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let surface = commands
        .spawn((
            crate::shellkit::role::Role::Opaque,
            Node {
                width: kit.m.px(WIDTH),
                max_width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::shellkit::glass::ground(crate::shellkit::glass::Density::Solid),
            crate::shellkit::glass::rim(),
            surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::POPOVER),
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let head = head(commands, kit, Phrase::ShellBellTitle.text(lang));
    commands.entity(surface).add_child(head);
    let table: Vec<(&BellItem, i64)> = state.bell.entries().collect();
    if state.social_notices.is_empty() && table.is_empty() {
        let empty = crate::shellkit::states::empty(
            commands,
            kit,
            glyph::BELL_SLASH,
            Phrase::ShellBellEmpty.text(lang),
            None,
        );
        let hint = super::super::parts::line(
            commands,
            kit,
            Phrase::ShellBellEmptyHint.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(hint).insert(TextLayout::new(
            Justify::Center,
            bevy::text::LineBreak::WordBoundary,
        ));
        commands.entity(empty).add_child(hint);
        commands.entity(surface).add_child(empty);
        return surface;
    }
    let list = commands
        .spawn((
            surfaces::SheetScroller,
            crate::shellkit::role::Role::Scroll,
            Pickable::default(),
            Node {
                flex_direction: FlexDirection::Column,
                max_height: kit.m.px(MAX_LIST),
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(8.0)),
                row_gap: kit.m.px(2.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .id();
    if !state.social_notices.is_empty() {
        section(commands, list, kit, Phrase::ShellBellWaiting.text(lang));
        for notice in &state.social_notices {
            let line = row(
                commands,
                kit,
                (notice.glyph, tokens::ACCENT),
                &notice.text,
                None,
                false,
            );
            commands.entity(list).add_child(line);
        }
    }
    if !table.is_empty() {
        section(commands, list, kit, Phrase::ShellBellTable.text(lang));
        let now = i64::try_from(super::super::parts::now_secs()).unwrap_or(0);
        for (i, (item, at)) in table.into_iter().enumerate() {
            let when = ago(now, at, lang);
            let line = row(
                commands,
                kit,
                mark(item),
                &item.sentence(lang),
                Some(&when),
                i < state.bell.fresh(),
            );
            commands.entity(list).add_child(line);
        }
    }
    let band = super::super::scrollbars::band_when_needed(commands, list, false);
    commands.entity(surface).add_child(band);
    surface
}

/// The list's head: the bell, gilt, and its name, over a hairline.
fn head(commands: &mut Commands, kit: Kit, title: &str) -> Entity {
    let head = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(12.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(glyph::BELL.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[mark, words]);
    head
}

/// A section's caption inside the list.
fn section(commands: &mut Commands, list: Entity, kit: Kit, text: &str) {
    let caption = super::super::parts::caption(commands, kit, text);
    commands.entity(caption).insert(Node {
        margin: UiRect::new(kit.m.px(6.0), Val::ZERO, kit.m.px(6.0), kit.m.px(4.0)),
        ..default()
    });
    commands.entity(list).add_child(caption);
}

/// One line: its mark on a round plate, what it says, and (for the table)
/// when; a line new since the bell was last opened is lit and dotted.
fn row(
    commands: &mut Commands,
    kit: Kit,
    (glyph, ink): (char, Color),
    text: &str,
    when: Option<&str>,
    fresh: bool,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(6.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if fresh {
                tokens::ACCENT.with_alpha(0.10)
            } else {
                Color::NONE
            }),
            Pickable::IGNORE,
        ))
        .id();
    let plate = kit.m.px(28.0);
    let disc = commands
        .spawn((
            Node {
                width: plate,
                height: plate,
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(ink.with_alpha(0.12)),
            BorderColor::all(ink.with_alpha(0.40)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.9),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(1.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = super::super::parts::line(commands, kit, text, kit.m.small, tokens::INK);
    commands.entity(words).add_child(said);
    if let Some(when) = when {
        let at = super::super::parts::line(commands, kit, when, kit.m.small * 0.9, tokens::MUTED);
        commands.entity(words).add_child(at);
    }
    commands.entity(line).add_children(&[disc, words]);
    if fresh {
        let dot = commands
            .spawn((
                Node {
                    width: kit.m.px(8.0),
                    height: kit.m.px(8.0),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(dot);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_line_has_its_own_mark() {
        let marks = [
            mark(&BellItem::Joined("Ole#9".into())).0,
            mark(&BellItem::Left("Ole#9".into())).0,
            mark(&BellItem::Filled).0,
        ];
        assert_ne!(marks[0], marks[1]);
        assert_ne!(marks[1], marks[2]);
        assert_ne!(marks[0], marks[2]);
    }
}
