//! The book’s fixed heading, chapter index and navigation.
use super::{ALBUM_ORDER, CloudArt, MemorialPress};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::surfaces;
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;

pub(super) fn header(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    art: Option<&CloudArt>,
) -> Entity {
    let lang = state.lobby.lang();
    let header = commands
        .spawn((
            Node {
                column_gap: kit.m.px(24.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let heading = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: kit.m.px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = super::pages::serif(commands, kit, "Baylee", kit.m.h1 * 1.25, true);
    let dedication = surfaces::prose(commands, kit, Phrase::MemorialStoryTitle.text(lang), false);
    let dates = surfaces::prose(commands, kit, Phrase::MemorialTogether.text(lang), true);
    commands
        .entity(heading)
        .add_children(&[title, dedication, dates]);
    commands.entity(header).add_child(heading);
    if let Some(art) = art {
        let cloud = commands
            .spawn((
                MaterialNode(art.0.clone()),
                Node {
                    width: kit.m.px(156.0),
                    height: kit.m.px(104.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(header).add_child(cloud);
    }
    header
}

pub(super) fn chapters(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let chapters = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let label = surfaces::prose(commands, kit, Phrase::MemorialChapters.text(lang), true);
    commands.entity(chapters).add_child(label);
    for (i, (title, _)) in (0..8_u8).zip(memorial::STORY) {
        let selected = usize::from(i) == state.memorial.chapter;
        let button = controls::button(
            commands,
            kit,
            &(i + 1).to_string(),
            if selected {
                Weight::Primary
            } else {
                Weight::Ghost
            },
            Live::Yes,
            None,
            (
                Press::Memorial(MemorialPress::Chapter(i)),
                Stop::item(ALBUM_ORDER.name, "chapters", i),
                Current(selected),
            ),
        );
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(title.text(lang));
        commands
            .entity(button)
            .insert(bevy::a11y::AccessibilityNode(access));
        commands.entity(chapters).add_child(button);
    }
    chapters
}

pub(super) fn footer(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let footer = commands
        .spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let back = controls::button(
        commands,
        kit,
        Phrase::MemorialChapterPrevious.text(lang),
        Weight::Ghost,
        if state.memorial.chapter > 0 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialFirstChapter.text(lang))
        },
        Some("←"),
        (
            Press::Memorial(MemorialPress::Back),
            Stop::new(ALBUM_ORDER.name, "back"),
        ),
    );
    let next = controls::button(
        commands,
        kit,
        Phrase::MemorialChapterNext.text(lang),
        Weight::Ghost,
        if state.memorial.chapter + 1 < memorial::CHAPTERS {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialLastChapter.text(lang))
        },
        Some("→"),
        (
            Press::Memorial(MemorialPress::Forward),
            Stop::new(ALBUM_ORDER.name, "forward"),
        ),
    );
    let close = controls::button(
        commands,
        kit,
        Phrase::SheetClose.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("Esc"),
        (
            Press::Memorial(MemorialPress::Close),
            Stop::new(ALBUM_ORDER.name, "close"),
        ),
    );
    commands.entity(footer).add_children(&[back, next, close]);
    footer
}
