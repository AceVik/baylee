//! Standardized `shellkit` header, numbered chapter strip, and symmetric chapter footer.
use super::{ALBUM_ORDER, CloudArt, MemorialPress, style};
use crate::lobby::{HoverHint, LobbyState, Press};
use crate::shellkit::controls::{Kit, Live};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{Frame, surfaces, tokens};
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;

pub(super) fn header(
    c: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    art: Option<&CloudArt>,
) -> Entity {
    let lang = state.lobby.lang();
    let mut close_access = accesskit::Node::new(accesskit::Role::Button);
    close_access.set_label(Phrase::SheetClose.text(lang));
    let close_bundle = (
        Press::Memorial(MemorialPress::Close),
        Stop::new(ALBUM_ORDER.name, "close"),
        bevy::a11y::AccessibilityNode(close_access),
    );
    let spec = surfaces::SheetHeader::new("Baylee")
        .subtitle_colored(Phrase::MemorialStoryTitle.text(lang), style::BRASS_LIGHT)
        .meta(Phrase::MemorialTogether.text(lang))
        .close(close_bundle);
    let spec = if let Some(art) = art {
        let medallion = c
            .spawn((
                Node {
                    width: kit.m.px(74.0),
                    height: kit.m.px(50.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(px_fixed(2.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.07, 0.11, 0.90)),
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
            .id();
        spec.lead(medallion)
    } else {
        spec.icon_colored('\u{f004}', style::BRASS_LIGHT)
    };
    surfaces::dialog_header(c, kit, spec)
}

pub(super) fn chapters(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let bar = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(6.0)),
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.07, 0.11, 0.72)),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let lead = style::label(c, kit, Phrase::MemorialChapters.text(lang), true);
    let pills = c
        .spawn((
            Node {
                column_gap: kit.m.px(5.0),
                row_gap: kit.m.px(4.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (i, (title, _)) in (0..8_u8).zip(memorial::STORY) {
        let selected = usize::from(i) == state.memorial.chapter;
        let chapter_title = title.text(lang);
        let button = style::step_button(
            c,
            kit,
            None,
            &(i + 1).to_string(),
            None,
            Live::Yes,
            selected,
            false,
            chapter_title,
            (
                Press::Memorial(MemorialPress::Chapter(i)),
                Stop::item(ALBUM_ORDER.name, "chapters", i),
            ),
        );
        c.entity(button)
            .insert((Current(selected), HoverHint(chapter_title.to_owned())));
        c.entity(pills).add_child(button);
    }
    c.entity(bar).add_children(&[lead, pills]);
    bar
}

pub(super) fn footer(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let compact_labels = kit.m.frame == Frame::Phone;
    let row = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px_fixed(kit.m.gap),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
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
