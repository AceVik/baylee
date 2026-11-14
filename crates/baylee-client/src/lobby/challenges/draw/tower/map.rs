//! The five-floor route, current encounter and committed improvements.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    season: &tower_api::Season,
) {
    heading(commands, parent, kit, Phrase::TowerRoute.text(lang));
    let cleared = model
        .tower
        .run
        .as_ref()
        .map_or(0, |run| run.progress.cleared);
    for floor in 0..5_u8 {
        let line = row(commands, parent, kit);
        commands
            .entity(line)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.flex_wrap = FlexWrap::NoWrap;
                node.min_height = kit.m.px(48.0);
                node.padding = UiRect::all(kit.m.px(6.0));
                node.border = UiRect::bottom(px(1));
            });
        commands
            .entity(line)
            .insert(BorderColor::all(tokens::BORDER));
        if floor == cleared {
            commands.entity(line).insert((
                BackgroundColor(tokens::MIST),
                BorderColor {
                    left: tokens::GOLD,
                    ..BorderColor::all(tokens::BORDER)
                },
            ));
            commands
                .entity(line)
                .entry::<Node>()
                .and_modify(|mut node| {
                    node.border.left = px(3);
                });
        }
        let ink = match floor.cmp(&cleared) {
            std::cmp::Ordering::Less => tokens::ACCENT,
            std::cmp::Ordering::Equal => tokens::GOLD,
            std::cmp::Ordering::Greater => tokens::MUTED,
        };
        let plaque = marker(commands, kit, floor, cleared, ink);
        commands
            .entity(plaque)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.width = kit.m.px(36.0);
                node.min_height = kit.m.px(36.0);
                node.justify_content = JustifyContent::Center;
                node.flex_shrink = 0.0;
            });
        commands.entity(line).add_child(plaque);
        if floor == 4 {
            commands.entity(plaque).insert(BorderColor {
                top: tokens::GOLD,
                ..BorderColor::all(tokens::BORDER)
            });
            commands
                .entity(plaque)
                .entry::<Node>()
                .and_modify(|mut node| {
                    node.border.top = px(3);
                });
        }
        let copy = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    row_gap: kit.m.px(4.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(copy);
        label(
            commands,
            copy,
            kit,
            words(&season.theme.floors[usize::from(floor)], lang),
            ink,
        );
        if let Some(track) = model
            .tower
            .run
            .as_ref()
            .and_then(|run| run.progress.route.get(usize::from(floor)))
        {
            label(
                commands,
                copy,
                kit,
                track_name(*track).text(lang),
                tokens::MUTED,
            );
        }
    }
}

fn marker(commands: &mut Commands, kit: Kit, floor: u8, cleared: u8, ink: Color) -> Entity {
    if floor < cleared {
        // The text face has no checkmark; draw it with the shipped icon font.
        crate::lobby::parts::badge_with(commands, kit, '\u{f00c}', "", ink)
    } else {
        crate::lobby::parts::badge(commands, kit, &(floor + 1).to_string(), ink)
    }
}
