//! The album uses the same focus trap and scrim as other shell sheets.
use super::{ALBUM_ORDER, MemorialPress};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{role::Role, surfaces, tokens};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

const PHOTOS: [(&str, f32); 5] = [
    ("memorial/baylee-1.jpg", 9.0 / 16.0),
    ("memorial/baylee-2.jpg", 16.0 / 9.0),
    ("memorial/baylee-3.jpg", 16.0 / 9.0),
    ("memorial/baylee-4.jpg", 9.0 / 16.0),
    ("memorial/baylee-5.jpg", 9.0 / 16.0),
];

#[allow(clippy::too_many_lines)] // One local album, laid out in reading order.
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
) {
    let lang = state.lobby.lang();
    let at = |id| Stop::new(ALBUM_ORDER.name, id);
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: kit.m.px(980.0),
                max_width: Val::Percent(94.0),
                max_height: Val::Percent(94.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(24.0)),
                row_gap: kit.m.px(12.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.055, 0.085, 0.105)),
            BorderColor::all(Color::srgb(0.78, 0.65, 0.39)),
            Press::Memorial(MemorialPress::Nothing),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new("Baylee"),
            crate::hud::tf_serif(kit.fonts, kit.m.h1 * 1.4, 500),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let dedication = surfaces::prose(commands, kit, Phrase::MemorialDedication.text(lang), false);
    let description = surfaces::prose(commands, kit, Phrase::MemorialStory.text(lang), false);
    let note = surfaces::prose(commands, kit, Phrase::MemorialDraft.text(lang), true);
    let (path, aspect) = PHOTOS[state.memorial.photo % PHOTOS.len()];
    let image = assets.map_or_else(Handle::default, |a| a.load(path));
    // The photograph's own ratio is preserved; portrait and landscape never stretch.
    let photo = commands
        .spawn((
            ImageNode::new(image),
            Node {
                height: Val::Vh(39.0),
                max_width: Val::Percent(100.0),
                aspect_ratio: Some(aspect),
                align_self: AlignSelf::Center,
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let navigation = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let prev = controls::button(
        commands,
        kit,
        Phrase::MemorialPrevious.text(lang),
        Weight::Ghost,
        Live::Yes,
        Some("←"),
        (Press::Memorial(MemorialPress::Previous), at("previous")),
    );
    commands.entity(navigation).add_child(prev);
    for i in 0..5_u8 {
        let picked = usize::from(i) == state.memorial.photo;
        let button = controls::button(
            commands,
            kit,
            &(i + 1).to_string(),
            if picked {
                Weight::Primary
            } else {
                Weight::Ghost
            },
            Live::Yes,
            None,
            (
                Press::Memorial(MemorialPress::Photo(i)),
                Stop::item(ALBUM_ORDER.name, "photos", i),
                Current(picked),
            ),
        );
        commands.entity(navigation).add_child(button);
    }
    let next = controls::button(
        commands,
        kit,
        Phrase::MemorialNext.text(lang),
        Weight::Ghost,
        Live::Yes,
        Some("→"),
        (Press::Memorial(MemorialPress::Next), at("next")),
    );
    commands.entity(navigation).add_child(next);
    let close = controls::button(
        commands,
        kit,
        Phrase::SheetClose.text(lang),
        Weight::Primary,
        Live::Yes,
        Some("Esc"),
        (Press::Memorial(MemorialPress::Close), at("close")),
    );
    commands.entity(surface).add_children(&[
        title,
        dedication,
        photo,
        navigation,
        description,
        note,
        close,
    ]);
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Memorial(MemorialPress::Close));
    commands.entity(root).add_child(scrim);
}
