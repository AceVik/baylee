//! Full-size photograph lightbox standardized on `shellkit` dialog surfaces.
mod geometry;
pub(in crate::lobby::memorial) use geometry::{PhotoPan, input, layout};
use geometry::{drag, restore};

use super::{ALBUM_ORDER, MemorialPress, photos, style};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::{
    Frame,
    controls::{Kit, Live},
    focus::Stop,
    metrics::px_fixed,
    surfaces, tokens,
};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

#[derive(Component)]
pub(in crate::lobby::memorial) struct PhotoViewport;
#[derive(Component)]
pub(in crate::lobby::memorial) struct LargePhoto;

pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
) {
    let lang = state.lobby.lang();
    let phone = kit.m.frame == Frame::Phone;
    let surface = surfaces::dialog(
        c,
        kit,
        Val::Percent(94.0),
        style::BRASS_BORDER,
        Press::Memorial(MemorialPress::Nothing),
    );
    c.entity(surface)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.height = if phone {
                Val::Percent(100.0)
            } else {
                Val::Percent(92.0)
            };
        });
    super::super::motion::attach(c, surface, false);

    let mut close_access = accesskit::Node::new(accesskit::Role::Button);
    close_access.set_label(Phrase::SheetClose.text(lang));
    let sub = format!(
        "Baylee · {:02} / {:02}",
        state.memorial.photo + 1,
        photos::PHOTOS.len()
    );
    let header = surfaces::dialog_header(
        c,
        kit,
        surfaces::SheetHeader::new(Phrase::MemorialPhotos.text(lang))
            .icon_colored('\u{f03e}', style::BRASS_LIGHT)
            .subtitle_colored(&sub, style::BRASS_LIGHT)
            .close((
                Press::Memorial(MemorialPress::PhotoClose),
                Stop::new(ALBUM_ORDER.name, "photo-close"),
                bevy::a11y::AccessibilityNode(close_access),
            )),
    );

    let stage = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let viewport = c
        .spawn((
            PhotoViewport,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                min_height: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style::PHOTO_LINER),
            BorderColor::all(style::BRASS_BORDER.with_alpha(0.55)),
        ))
        .observe(drag)
        .id();
    let path = format!("memorial/full/baylee-{}.jpg", state.memorial.photo + 1);
    let image = c
        .spawn((
            LargePhoto,
            Visibility::Hidden,
            ImageNode::new(assets.map_or_else(Handle::default, |a| a.load(path))),
            Node {
                width: px_fixed(1.0),
                height: px_fixed(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    restore(c, image);
    c.entity(viewport).add_child(image);
    c.entity(stage).add_child(viewport);

    let toolbar = toolbar(c, state, kit);
    c.entity(surface).add_children(&[header, stage, toolbar]);

    let scrim = surfaces::scrim(
        c,
        kit,
        tokens::z::TOAST + 1,
        Press::Memorial(MemorialPress::PhotoClose),
        surface,
    );
    c.entity(root).add_child(scrim);
}

fn toolbar(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let toolbar = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(12.0)),
                column_gap: px_fixed(kit.m.gap),
                row_gap: kit.m.px(8.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.18)),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let navigation = photos::navigation(c, state, kit, false);
    let zoom = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let out = style::step_button(
        c,
        kit,
        Some('\u{f068}'),
        "",
        None,
        if state.memorial.zoom > 0 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialZoomMinimum.text(lang))
        },
        false,
        false,
        Phrase::MemorialZoomOut.text(lang),
        (
            Press::Memorial(MemorialPress::ZoomOut),
            Stop::new(ALBUM_ORDER.name, "zoom-out"),
        ),
    );
    let percent = style::pill_counter(
        c,
        kit,
        &format!("{} %", 100 + u16::from(state.memorial.zoom) * 50),
    );
    let into = style::step_button(
        c,
        kit,
        Some('\u{f067}'),
        "",
        None,
        if state.memorial.zoom < 6 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialZoomMaximum.text(lang))
        },
        false,
        false,
        Phrase::MemorialZoomIn.text(lang),
        (
            Press::Memorial(MemorialPress::ZoomIn),
            Stop::new(ALBUM_ORDER.name, "zoom-in"),
        ),
    );
    let fit_word = Phrase::MemorialFit.text(lang);
    let fit = style::step_button(
        c,
        kit,
        Some('\u{f065}'),
        fit_word,
        None,
        Live::Yes,
        false,
        false,
        fit_word,
        (
            Press::Memorial(MemorialPress::Fit),
            Stop::new(ALBUM_ORDER.name, "fit"),
        ),
    );
    c.entity(zoom).add_children(&[out, percent, into, fit]);
    c.entity(toolbar).add_children(&[navigation, zoom]);
    toolbar
}
