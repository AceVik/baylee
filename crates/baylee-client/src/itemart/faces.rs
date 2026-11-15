//! A portrait fits only in the unused band; it never replaces rules or a print.
use baylee_client_core::images::ImageKey;
use bevy::prelude::*;

fn square([x0, y0, x1, y1]: [f32; 4]) -> Rect {
    let side = ((x1 - x0).min(y1 - y0) - 0.04).max(0.0);
    Rect::from_center_size(
        Vec2::new(f32::midpoint(x0, x1), f32::midpoint(y0, y1)),
        Vec2::splat(side),
    )
}

pub(crate) fn ui(
    commands: &mut Commands,
    card: Entity,
    key: Option<ImageKey>,
    band: Option<[f32; 4]>,
    width: f32,
) {
    let (Some(goods), Some(band)) = (super::identity::of(key), band) else {
        return;
    };
    let rect = square(band);
    commands.spawn((
        super::Portrait(goods),
        Pickable::IGNORE,
        ChildOf(card),
        Node {
            position_type: PositionType::Absolute,
            left: px(rect.min.x * width),
            top: px(rect.min.y * width),
            width: px(rect.width() * width),
            height: px(rect.height() * width),
            ..default()
        },
    ));
}

pub(crate) fn world(
    commands: &mut Commands,
    card: Entity,
    key: Option<ImageKey>,
    band: [f32; 4],
    veil: u32,
) -> Option<Entity> {
    let goods = super::identity::of(key)?;
    let rect = square(band);
    let center = rect.center();
    // The table's physical card aspect differs slightly from its text grid.
    // Fit in both directions but keep the original portrait truly square.
    let side = (rect.width() * baylee_client_core::layout::CARD_WIDTH)
        .min(rect.height() * crate::table::DOWN_THE_CARD);
    Some(
        commands
            .spawn((
                super::world::Portrait {
                    goods: super::Portrait(goods),
                    veil: u8::try_from(veil & 7).expect("three bits"),
                },
                Pickable::IGNORE,
                ChildOf(card),
                Transform::from_xyz(
                    (center.x - 0.5) * baylee_client_core::layout::CARD_WIDTH,
                    baylee_client_core::layout::CARD_HEIGHT * 0.5
                        - center.y * crate::table::DOWN_THE_CARD,
                    crate::face::WORLD_FACE_Z + 0.001,
                )
                .with_scale(Vec3::new(side, side, 1.0)),
            ))
            .id(),
    )
}

#[cfg(test)]
mod tests;
