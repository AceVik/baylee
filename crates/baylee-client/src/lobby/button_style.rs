//! Quiet primary surfaces share one shader; action icons use bundled Font Awesome.
use super::Press;
use crate::ambience::{AmbienceMaterial, AmbienceParams};
use crate::hud::{UiFonts, icon_tf, palette};
use bevy::prelude::*;
use bevy::ui::px;

#[derive(Component)]
pub(super) struct Primary;
#[derive(Component)]
pub(super) struct Styled;

pub(crate) fn primary(commands: &mut Commands, id: Entity) {
    let fill = Color::srgb(0.14, 0.24, 0.33);
    commands.entity(id).insert((
        Primary,
        BackgroundColor(fill),
        BorderColor::all(Color::srgb(0.52, 0.53, 0.48)),
        crate::ambience::Feel::new(fill),
    ));
}

pub(crate) fn icon(commands: &mut Commands, fonts: &UiFonts, id: Entity, press: Press, size: f32) {
    // Only Font Awesome glyphs here; no new Mana/Wizards symbols are introduced.
    let glyph = match press {
        Press::BrowseHistory | Press::DeckHistory(_) => '\u{f1da}',
        Press::BrowseHouse => '\u{f015}',
        Press::SaveDeck => '\u{f0c7}',
        Press::NewDeck => '\u{f067}',
        Press::ClearDeck | Press::DeleteDeck(_) => '\u{f2ed}',
        Press::ToggleStatistics => '\u{f080}',
        Press::ChooseCommander(_) => '\u{f521}',
        Press::ToggleDeckActions => '\u{f142}',
        Press::LeaveGateway => '\u{f053}',
        _ => return,
    };
    let icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            icon_tf(fonts, size),
            TextColor(palette::DOCK_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(id).insert_children(0, &[icon]);
}

pub(super) fn materialize(
    mut commands: Commands,
    nodes: Query<Entity, (With<Primary>, Without<Styled>)>,
    mut cached: Local<Option<Handle<AmbienceMaterial>>>,
    materials: Option<ResMut<Assets<AmbienceMaterial>>>,
    prefs: Res<crate::prefs::Prefs>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    if nodes.is_empty() && !prefs.is_changed() {
        return;
    }
    let handle = cached.get_or_insert_with(|| {
        materials.add(AmbienceMaterial {
            params: AmbienceParams {
                low: Color::srgb(0.14, 0.24, 0.33).to_linear().to_vec4(),
                high: Color::srgb(0.4, 0.54, 0.62).to_linear().to_vec4(),
                energy: 1.0,
                seed: 0.0,
                aspect: 3.0,
                pad: 1.0,
            },
        })
    });
    if let Some(mut material) = materials.get_mut(&*handle) {
        material.params.energy = if prefs.all().reduce_motion { 0.0 } else { 1.0 };
    }
    for id in &nodes {
        let surface = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(1),
                    right: px(1),
                    top: px(1),
                    bottom: px(1),
                    ..default()
                },
                MaterialNode(handle.clone()),
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(id)
            .insert(Styled)
            .insert_children(0, &[surface]);
    }
}
