//! Reveal a freshly drawn sample once its real layout is known.

use crate::lobby::LobbyState;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct SampleHand;

pub(crate) fn reveal(
    mut state: ResMut<LobbyState>,
    hands: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<SampleHand>>,
    parents: Query<&ChildOf>,
    mut scrollers: Query<(
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        &mut ScrollPosition,
    )>,
) {
    if !state.build.reveal_hand {
        return;
    }
    if state.build.hand.is_empty() {
        state.bypass_change_detection().build.reveal_hand = false;
        return;
    }
    for (hand, size, at) in &hands {
        if size.size().y <= 0.0 {
            continue;
        }
        for ancestor in parents.iter_ancestors(hand) {
            let Ok((node, view, origin, mut scroll)) = scrollers.get_mut(ancestor) else {
                continue;
            };
            // Every Node has a ScrollPosition, including non-scrolling wrappers.
            if node.overflow.y != OverflowAxis::Scroll || view.size().y <= 0.0 {
                continue;
            }
            let scale = view.inverse_scale_factor();
            let top = (at.translation.y - size.size().y * 0.5 - origin.translation.y
                + view.size().y * 0.5)
                * scale;
            let height = size.size().y * scale;
            let visible = view.size().y * scale;
            let delta = if top < 0.0 || height >= visible {
                top
            } else {
                (top + height - visible).max(0.0)
            };
            let end = ((view.content_size().y - view.size().y) * scale).max(0.0);
            let want = (scroll.y + delta).clamp(0.0, end);
            if (scroll.y - want).abs() > 0.5 {
                scroll.y = want;
            }
            state.bypass_change_detection().build.reveal_hand = false;
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_reveals_the_sample_once_without_fighting_later_scrolling() {
        let mut app = App::new();
        let mut state = LobbyState::default();
        state.build.hand = vec![0];
        state.build.reveal_hand = true;
        app.insert_resource(state).add_systems(Update, reveal);
        let list = app
            .world_mut()
            .spawn((
                Node {
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition(Vec2::new(0.0, 100.0)),
                ComputedNode {
                    size: Vec2::new(200.0, 300.0),
                    content_size: Vec2::new(200.0, 700.0),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::new(100.0, 150.0)),
            ))
            .id();
        let wrapper = app
            .world_mut()
            .spawn((
                Node::default(),
                ChildOf(list),
                ComputedNode {
                    size: Vec2::new(200.0, 700.0),
                    ..default()
                },
                UiGlobalTransform::default(),
            ))
            .id();
        app.world_mut().spawn((
            SampleHand,
            ChildOf(wrapper),
            ComputedNode {
                size: Vec2::new(200.0, 100.0),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(100.0, 400.0)),
        ));
        app.update();
        assert!((app.world().get::<ScrollPosition>(list).unwrap().y - 250.0).abs() < 0.01);
        app.world_mut().get_mut::<ScrollPosition>(list).unwrap().y = 0.0;
        app.update();
        assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
    }
}
