//! A quiet title, numbered bookmarks and one navigation row below the writing.
use super::{ALBUM_ORDER, CloudArt, MemorialPress, style};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::controls::{Kit, Live};
use crate::shellkit::focus::{Current, Stop};
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;

pub(super) fn header(
    c: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    art: Option<&CloudArt>,
) -> Entity {
    let lang = state.lobby.lang();
    let header = c
        .spawn((
            Node {
                column_gap: kit.m.px(20.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(art) = art {
        let cloud = c
            .spawn((
                MaterialNode(art.0.clone()),
                Node {
                    width: kit.m.px(126.0),
                    height: kit.m.px(84.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        c.entity(header).add_child(cloud);
    }
    let heading = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = super::pages::serif(c, kit, "Baylee", kit.m.h1 * 1.2, true);
    let dedication = style::label(c, kit, Phrase::MemorialStoryTitle.text(lang), false);
    let dates = style::label(c, kit, Phrase::MemorialTogether.text(lang), true);
    c.entity(heading).add_children(&[title, dedication, dates]);
    let close = style::button(
        c,
        kit,
        "×",
        Live::Yes,
        false,
        (
            Press::Memorial(MemorialPress::Close),
            Stop::new(ALBUM_ORDER.name, "close"),
        ),
    );
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(Phrase::SheetClose.text(lang));
    c.entity(close)
        .insert(bevy::a11y::AccessibilityNode(access));
    c.entity(header).add_children(&[heading, close]);
    header
}

pub(super) fn chapters(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let row = c
        .spawn((
            Node {
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let label = style::label(c, kit, Phrase::MemorialChapters.text(lang), true);
    c.entity(row).add_child(label);
    for (i, (title, _)) in (0..8_u8).zip(memorial::STORY) {
        let selected = usize::from(i) == state.memorial.chapter;
        let button = style::button(
            c,
            kit,
            &(i + 1).to_string(),
            Live::Yes,
            selected,
            (
                Press::Memorial(MemorialPress::Chapter(i)),
                Stop::item(ALBUM_ORDER.name, "chapters", i),
            ),
        );
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(title.text(lang));
        c.entity(button)
            .insert((Current(selected), bevy::a11y::AccessibilityNode(access)));
        c.entity(row).add_child(button);
    }
    row
}

pub(super) fn footer(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let row = c
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(12.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let back = style::button(
        c,
        kit,
        "←",
        if state.memorial.chapter > 0 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialFirstChapter.text(lang))
        },
        false,
        (
            Press::Memorial(MemorialPress::Back),
            Stop::new(ALBUM_ORDER.name, "back"),
        ),
    );
    let count = style::label(
        c,
        kit,
        &format!(
            "{}   ·   {:02} / 08",
            Phrase::MemorialChapters.text(lang),
            state.memorial.chapter + 1
        ),
        true,
    );
    let next = style::button(
        c,
        kit,
        "→",
        if state.memorial.chapter + 1 < memorial::CHAPTERS {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialLastChapter.text(lang))
        },
        false,
        (
            Press::Memorial(MemorialPress::Forward),
            Stop::new(ALBUM_ORDER.name, "forward"),
        ),
    );
    for (e, p) in [
        (back, Phrase::MemorialChapterPrevious),
        (next, Phrase::MemorialChapterNext),
    ] {
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(p.text(lang));
        c.entity(e).insert(bevy::a11y::AccessibilityNode(access));
    }
    c.entity(row).add_children(&[back, count, next]);
    row
}
