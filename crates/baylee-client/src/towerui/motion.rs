//! Disposable icon/coin travel outside layout. Never moves an interactive slot.
use super::{Goods, tokens};
use bevy::prelude::*;

#[derive(Component)]
struct Motion {
    from: Vec2,
    to: Vec2,
    age: f32,
    delay: f32,
    duration: f32,
    side: f32,
    pulse: bool,
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(Update, animate);
}

pub(crate) fn item(
    commands: &mut Commands,
    goods: Goods,
    from: Vec2,
    to: Vec2,
    fusion: bool,
) -> Entity {
    let side = if fusion { 60.0 } else { 52.0 };
    let entity = super::drag_ghost(commands, goods, from, side);
    commands.entity(entity).insert(Motion {
        from,
        to,
        age: 0.0,
        delay: 0.0,
        duration: if fusion { 0.85 } else { 0.28 },
        side,
        pulse: false,
    });
    entity
}

pub(crate) fn coin(
    commands: &mut Commands,
    fonts: &crate::hud::UiFonts,
    from: Vec2,
    to: Vec2,
    ordinal: u8,
) -> Entity {
    let side = 24.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(from.x - side * 0.5),
                top: px(from.y - side * 0.5),
                width: px(side),
                height: px(side),
                ..default()
            },
            Text::new(crate::hud::glyph::COINS.to_string()),
            crate::hud::icon_tf(fonts, side),
            TextColor(tokens::GOLD),
            crate::shellkit::surfaces::popover_shadow(),
            GlobalZIndex(tokens::z::TOOLTIP + 1),
            Pickable::IGNORE,
            Visibility::Hidden,
            Motion {
                from,
                to,
                age: 0.0,
                delay: f32::from(ordinal) * 0.055,
                duration: 0.42,
                side,
                pulse: false,
            },
        ))
        .id()
}

pub(crate) fn fusion(commands: &mut Commands, goods: Goods, position: Vec2) -> Entity {
    let side = 76.0;
    let entity = super::drag_ghost(commands, goods, position, side);
    commands.entity(entity).insert((
        Motion {
            from: position,
            to: position,
            age: 0.0,
            delay: 0.75,
            duration: 0.6,
            side,
            pulse: true,
        },
        Visibility::Hidden,
        match goods {
            Goods::Relic(item) => match item.rarity {
                super::Rarity::Common => crate::itemart::Emphasis::Owned,
                super::Rarity::Uncommon => crate::itemart::Emphasis::Uncommon,
                super::Rarity::Rare => crate::itemart::Emphasis::Rare,
                super::Rarity::MythicRare => crate::itemart::Emphasis::Mythic,
            },
            _ => crate::itemart::Emphasis::Owned,
        },
    ));
    entity
}

fn animate(
    mut commands: Commands,
    clock: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut nodes: Query<(
        Entity,
        &mut Motion,
        &mut Node,
        &mut UiTransform,
        &mut Visibility,
    )>,
) {
    let reduced = prefs
        .as_ref()
        .is_some_and(|prefs| prefs.all().reduce_motion);
    for (entity, mut motion, mut node, mut transform, mut visibility) in &mut nodes {
        motion.age += clock.delta_secs();
        if reduced || motion.age >= motion.delay + motion.duration {
            commands.entity(entity).try_despawn();
            continue;
        }
        if motion.age < motion.delay {
            continue;
        }
        *visibility = Visibility::Inherited;
        let progress = ((motion.age - motion.delay) / motion.duration).clamp(0.0, 1.0);
        let smooth = progress * progress * (3.0 - 2.0 * progress);
        let arc = (std::f32::consts::PI * progress).sin();
        let point = motion.from.lerp(motion.to, smooth)
            - Vec2::Y * arc * if motion.pulse { 0.0 } else { 32.0 };
        node.left = px(point.x - motion.side * 0.5);
        node.top = px(point.y - motion.side * 0.5);
        transform.scale = Vec2::splat(if motion.pulse {
            1.0 + 0.22 * arc
        } else {
            1.0 + 0.08 * arc
        });
    }
}
