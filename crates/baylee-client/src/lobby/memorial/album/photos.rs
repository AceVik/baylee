//! Double-matted gallery photograph frame and symmetric photo navigation bar.
use super::{ALBUM_ORDER, MemorialPress, style};
use crate::lobby::{HoverHint, LobbyState, Press};
use crate::shellkit::{
    Frame,
    controls::{Kit, Live},
    focus::Stop,
    metrics::px_fixed,
    role::Role,
    tokens,
};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

pub(super) const PHOTOS: [(&str, f32); baylee_client_core::memorial::PHOTOS] = [
    ("memorial/baylee-1.jpg", 9.0 / 16.0),
    ("memorial/baylee-2.jpg", 16.0 / 9.0),
    ("memorial/baylee-3.jpg", 16.0 / 9.0),
    ("memorial/baylee-4.jpg", 9.0 / 16.0),
    ("memorial/baylee-5.jpg", 9.0 / 16.0),
    ("memorial/baylee-6.jpg", 16.0 / 9.0),
    ("memorial/baylee-7.jpg", 16.0 / 9.0),
    ("memorial/baylee-8.jpg", 16.0 / 9.0),
    ("memorial/baylee-9.jpg", 9.0 / 16.0),
    ("memorial/baylee-10.jpg", 16.0 / 9.0),
];

fn photo_frame(
    c: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
    open_hint: &str,
) -> Entity {
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(open_hint);
    let frame = c
        .spawn((
            crate::ui_button::UiButton,
            Role::Hit,
            Press::Memorial(MemorialPress::PhotoOpen),
            Stop::new(ALBUM_ORDER.name, "photo"),
            HoverHint(open_hint.to_owned()),
            bevy::a11y::AccessibilityNode(access),
            UiTransform::default(),
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(kit.m.px(10.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                border: UiRect::all(px_fixed(1.0)),
                ..default()
            },
            style::photo_shadow(),
            BackgroundColor(style::PHOTO_MAT),
            BorderColor::all(style::BRASS_BORDER),
            crate::ambience::Feel::tinting_to(style::PHOTO_MAT, style::PHOTO_MAT_HOT),
            crate::ambience::FeelBorder {
                rest: style::BRASS_BORDER,
                hot: style::BRASS_LIGHT,
                press: style::BRASS,
            },
            crate::ambience::FeelShadow,
        ))
        .id();
    let liner = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                min_height: px_fixed(0.0),
                padding: UiRect::all(px_fixed(2.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style::PHOTO_LINER),
            BorderColor::all(style::BRASS_LIGHT.with_alpha(0.28)),
            Pickable::IGNORE,
        ))
        .id();
    let photo = c
        .spawn((
            BackgroundColor(style::PHOTO_LINER),
            Node {
                height: Val::Percent(100.0),
                width: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    super::super::preview::attach(c, photo, state.memorial.photo, assets.is_some());
    super::super::motion::attach(c, photo, true);
    c.entity(liner).add_child(photo);
    c.entity(frame).add_child(liner);
    frame
}

fn photo_caption(c: &mut Commands, kit: Kit, title_str: &str, open_hint: &str) -> Entity {
    let col = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = style::label(c, kit, title_str, false);
    let hint_row = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let zoom_icon = c
        .spawn((
            Text::new("\u{f00e}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(style::BRASS_LIGHT.with_alpha(0.85)),
            Pickable::IGNORE,
        ))
        .id();
    let hint = style::label(c, kit, open_hint, true);
    c.entity(hint_row).add_children(&[zoom_icon, hint]);
    c.entity(col).add_children(&[caption, hint_row]);
    col
}

pub(super) fn draw(
    c: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
    stacked: bool,
) {
    let lang = state.lobby.lang();
    let side = c
        .spawn((
            Node {
                width: if stacked {
                    Val::Percent(100.0)
                } else {
                    Val::Percent(32.0)
                },
                height: if stacked {
                    Val::Percent(40.0)
                } else {
                    Val::Percent(100.0)
                },
                flex_shrink: 0.0,
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let open_hint = Phrase::MemorialPhotoOpen.text(lang);
    let frame = photo_frame(c, state, kit, assets, open_hint);
    let caption_col = photo_caption(c, kit, Phrase::MemorialPhotos.text(lang), open_hint);
    let nav = navigation(c, state, kit, true);
    c.entity(side).add_children(&[frame, caption_col, nav]);
    c.entity(parent).add_child(side);
}

pub(super) fn navigation(c: &mut Commands, state: &LobbyState, kit: Kit, stretch: bool) -> Entity {
    let lang = state.lobby.lang();
    let compact_labels = kit.m.frame == Frame::Phone || !stretch;
    let row = c
        .spawn((
            Node {
                width: if stretch {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                },
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let prev_word = Phrase::MemorialPrevious.text(lang);
    let prev = style::step_button(
        c,
        kit,
        Some('\u{f060}'),
        if compact_labels { "" } else { prev_word },
        None,
        Live::Yes,
        false,
        stretch,
        prev_word,
        (
            Press::Memorial(MemorialPress::Previous),
            Stop::new(ALBUM_ORDER.name, "previous"),
        ),
    );
    let count = style::pill_counter(
        c,
        kit,
        &format!("{:02} / {:02}", state.memorial.photo + 1, PHOTOS.len()),
    );
    let next_word = Phrase::MemorialNext.text(lang);
    let next = style::step_button(
        c,
        kit,
        None,
        if compact_labels { "" } else { next_word },
        Some('\u{f061}'),
        Live::Yes,
        false,
        stretch,
        next_word,
        (
            Press::Memorial(MemorialPress::Next),
            Stop::new(ALBUM_ORDER.name, "next"),
        ),
    );
    c.entity(row).add_children(&[prev, count, next]);
    row
}
