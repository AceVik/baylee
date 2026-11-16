//! Symmetric 3-column gallery header, centered chapter ribbon, and symmetric footer.
use super::{ALBUM_ORDER, CloudArt, MemorialPress, style};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::controls::{Kit, Live};
use crate::shellkit::focus::Stop;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{Frame, surfaces, tokens};
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;

fn header_medallion(c: &mut Commands, kit: Kit, art: Option<&CloudArt>) -> Entity {
    if let Some(art) = art {
        c.spawn((
            Node {
                width: kit.m.px(52.0),
                height: kit.m.px(36.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px_fixed(2.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.055, 0.092, 0.86)),
            BorderColor::all(style::BRASS_BORDER),
            surfaces::elevation_control(),
            Pickable::IGNORE,
        ))
        .with_child((
            MaterialNode(art.cloud.clone()),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
    } else {
        c.spawn((
            Node {
                width: kit.m.px(34.0),
                height: kit.m.px(34.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(style::BRASS.with_alpha(0.14)),
            BorderColor::all(style::BRASS_BORDER),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new("\u{f004}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(style::BRASS_LIGHT),
            Pickable::IGNORE,
        ))
        .id()
    }
}

fn header_center(
    c: &mut Commands,
    kit: Kit,
    lang: baylee_client_core::i18n::Lang,
    phone: bool,
) -> Entity {
    let center_col = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(2.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = c
        .spawn((
            Text::new("Baylee"),
            crate::hud::tf_bold(kit.fonts, kit.m.head * 1.16),
            TextColor(style::TITLE_INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let subtitle = c
        .spawn((
            Text::new(Phrase::MemorialStoryTitle.text(lang)),
            crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small)),
            TextColor(style::BRASS_LIGHT),
            TextLayout::justify(Justify::Center),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    c.entity(center_col).add_children(&[title, subtitle]);
    if !phone {
        let dates = c
            .spawn((
                Text::new(Phrase::MemorialTogether.text(lang)),
                crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.88)),
                TextColor(tokens::MUTED),
                TextLayout::justify(Justify::Center),
                bevy::ui::LayoutConfig {
                    use_rounding: false,
                },
                Pickable::IGNORE,
            ))
            .id();
        c.entity(center_col).add_child(dates);
    }
    center_col
}

pub(super) fn header(
    c: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    art: Option<&CloudArt>,
) -> Entity {
    let lang = state.lobby.lang();
    let phone = kit.m.frame == Frame::Phone;
    let head = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(
                    kit.m.px(if phone { 14.0 } else { 22.0 }),
                    kit.m.px(if phone { 8.0 } else { 14.0 }),
                ),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(12.0),
                flex_shrink: 0.0,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(style::BRASS_BORDER.with_alpha(0.32)),
            Pickable::IGNORE,
        ))
        .id();

    let left_slot = c
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if !phone {
        let cameo = header_medallion(c, kit, art);
        c.entity(left_slot).add_child(cameo);
    }

    let center_col = header_center(c, kit, lang, phone);

    let mut close_access = accesskit::Node::new(accesskit::Role::Button);
    close_access.set_label(Phrase::SheetClose.text(lang));
    let close_btn = surfaces::sheet_close_button(
        c,
        kit,
        (
            Press::Memorial(MemorialPress::Close),
            Stop::new(ALBUM_ORDER.name, "close"),
            bevy::a11y::AccessibilityNode(close_access),
        ),
    );
    let right_slot = c
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_child(close_btn)
        .id();

    c.entity(head)
        .add_children(&[left_slot, center_col, right_slot]);
    head
}

pub(super) fn chapters(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let bar = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(16.0), kit.m.px(6.0)),
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(4.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.018, 0.036, 0.062, 0.34)),
            BorderColor::all(tokens::BORDER.with_alpha(0.46)),
            Pickable::IGNORE,
        ))
        .id();
    for (i, (title, _)) in (0..8_u8).zip(memorial::STORY) {
        let selected = usize::from(i) == state.memorial.chapter;
        let button = style::chapter_pill(
            c,
            kit,
            &(i + 1).to_string(),
            selected,
            title.text(lang),
            (
                Press::Memorial(MemorialPress::Chapter(i)),
                Stop::item(ALBUM_ORDER.name, "chapters", i),
            ),
        );
        c.entity(bar).add_child(button);
    }
    bar
}

pub(super) fn footer(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let phone = kit.m.frame == Frame::Phone;
    let compact_labels = matches!(kit.m.frame, Frame::Phone | Frame::Compact);
    let row = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(
                    kit.m.px(if phone { 14.0 } else { 22.0 }),
                    kit.m.px(if phone { 8.0 } else { 11.0 }),
                ),
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.018, 0.036, 0.062, 0.38)),
            BorderColor::all(style::BRASS_BORDER.with_alpha(0.28)),
            Pickable::IGNORE,
        ))
        .id();
    let prev_label = Phrase::MemorialChapterPrevious.text(lang);
    let back = style::step_button(
        c,
        kit,
        Some('\u{f060}'),
        if compact_labels { "" } else { prev_label },
        None,
        if state.memorial.chapter > 0 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialFirstChapter.text(lang))
        },
        false,
        true,
        prev_label,
        (
            Press::Memorial(MemorialPress::Back),
            Stop::new(ALBUM_ORDER.name, "back"),
        ),
    );
    let count = style::pill_counter(
        c,
        kit,
        &format!(
            "{:02} / {:02}",
            state.memorial.chapter + 1,
            memorial::CHAPTERS
        ),
    );
    let next_label = Phrase::MemorialChapterNext.text(lang);
    let next = style::step_button(
        c,
        kit,
        None,
        if compact_labels { "" } else { next_label },
        Some('\u{f061}'),
        if state.memorial.chapter + 1 < memorial::CHAPTERS {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialLastChapter.text(lang))
        },
        false,
        true,
        next_label,
        (
            Press::Memorial(MemorialPress::Forward),
            Stop::new(ALBUM_ORDER.name, "forward"),
        ),
    );
    c.entity(row).add_children(&[back, count, next]);
    row
}
