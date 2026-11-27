//! The actual sidebar width can change when friends slide into the lobby.
use super::*;
use crate::shellkit::{ShellMetrics, TextSize, Viewport};

#[test]
fn a_narrow_desktop_hero_keeps_deck_words_and_actions_inside_its_lane() {
    for lang in [Lang::En, Lang::De] {
        let (mut app, fonts) = crate::face::tests::layout_app();
        let mut state = LobbyState::default();
        state.lobby.set_lang(lang);
        state.lobby.apply(client_core::lobby::LobbyEvent::LoggedIn {
            token: "local-test".into(),
            username: None,
        });
        state
            .lobby
            .apply(client_core::lobby::LobbyEvent::Decks(vec![
                client_core::lobby::DeckSummary {
                    id: "hero-deck".into(),
                    name: "Breya, Etherium Shaper".into(),
                    copies: 101,
                    identity: "WUBR".into(),
                    ..default()
                },
            ]));
        let kit = Kit {
            fonts: &fonts,
            m: ShellMetrics::of(Viewport::desktop(1280.0, 800.0), TextSize::M),
            german: lang == Lang::De,
        };
        let mut commands = app.world_mut().commands();
        let lane = commands
            .spawn(Node {
                width: px_fixed(260.0),
                height: px_fixed(800.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        hero(&mut commands, lane, &state, Metrics::of(1280.0), kit);
        app.world_mut().flush();
        for _ in 0..4 {
            app.update();
        }
        let world = app.world_mut();
        let node = world.get::<ComputedNode>(lane).unwrap();
        let at = world.get::<bevy::ui::UiGlobalTransform>(lane).unwrap();
        let bounds = Rect::from_center_size(at.translation, node.size());
        assert!(bounds.width() > 250.0);
        let mut seen = 0;
        for (text, node, at) in world
            .query::<(&Text, &ComputedNode, &bevy::ui::UiGlobalTransform)>()
            .iter(world)
        {
            if text.0.is_empty() {
                continue;
            }
            seen += 1;
            let rect = Rect::from_center_size(at.translation, node.size());
            assert!(
                rect.min.x >= bounds.min.x - 0.5 && rect.max.x <= bounds.max.x + 0.5,
                "{}: {rect:?} outside {bounds:?}",
                text.0
            );
        }
        assert!(seen >= 6, "real caption, deck name, metadata and controls");
    }
}

#[test]
fn play_lane_stable_gutter_stands_inside_the_column_gap() {
    use bevy::ui_widgets::Scrollbar;

    for (width, height) in [(1280.0, 720.0), (1600.0, 938.0), (1920.0, 1080.0)] {
        let (mut app, fonts) = crate::face::tests::layout_app();
        let mut state = LobbyState::default();
        state.lobby.apply(client_core::lobby::LobbyEvent::LoggedIn {
            token: "local-test".into(),
            username: None,
        });
        let kit = Kit {
            fonts: &fonts,
            m: ShellMetrics::of(Viewport::desktop(width, height), TextSize::M),
            german: false,
        };
        let scrolled = Scrolled::default();
        let mut commands = app.world_mut().commands();
        let root = commands
            .spawn(Node {
                width: px_fixed(width),
                height: px_fixed(height),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        draw(
            &mut commands,
            root,
            &state,
            Metrics::of(width),
            kit,
            &scrolled,
        );
        app.world_mut().flush();
        for _ in 0..4 {
            app.update();
        }
        let world = app.world_mut();
        let (lane_entity, lane_rect) = world
            .query::<(
                Entity,
                &Scrollable,
                &ComputedNode,
                &bevy::ui::UiGlobalTransform,
            )>()
            .iter(world)
            .find(|(_, scrollable, ..)| scrollable.0 == List::Discovery)
            .map(|(e, _, node, at)| (e, Rect::from_center_size(at.translation, node.size())))
            .expect("Play activity lane laid out");
        let band_entity = world.get::<ChildOf>(lane_entity).unwrap().parent();
        let body_entity = world.get::<ChildOf>(band_entity).unwrap().parent();
        let tables_rect = world
            .query::<(&ChildOf, &Role, &ComputedNode, &bevy::ui::UiGlobalTransform)>()
            .iter(world)
            .find(|(parent, role, ..)| parent.parent() == body_entity && **role == Role::Panel)
            .map(|(_, _, node, at)| Rect::from_center_size(at.translation, node.size()))
            .expect("Tables panel laid out beside Play lane");
        let (track_rect, track_node, is_stable) = world
            .query::<(
                &Scrollbar,
                &ComputedNode,
                &bevy::ui::UiGlobalTransform,
                &Node,
                Has<crate::lobby::scrollbars::StableGutter>,
            )>()
            .iter(world)
            .find(|(bar, ..)| bar.target == lane_entity)
            .map(|(_, computed, at, node, stable)| {
                (
                    Rect::from_center_size(at.translation, computed.size()),
                    node.clone(),
                    stable,
                )
            })
            .expect("Play lane scrollbar track");

        assert!(is_stable, "Play lane keeps its stable scrollbar gutter");
        assert_eq!(track_node.display, Display::Flex);
        let gap = tables_rect.min.x - lane_rect.max.x;
        assert!(
            (gap - kit.m.body).abs() < 1.0,
            "{width}x{height}: Play-to-Tables gap {gap:.1} should match body gap {:.1}",
            kit.m.body
        );
        assert!(
            track_rect.min.x >= lane_rect.max.x - 0.5
                && track_rect.max.x <= tables_rect.min.x + 0.5,
            "{width}x{height}: scrollbar track {track_rect:?} sits inside the gap [{:.1}, {:.1}]",
            lane_rect.max.x,
            tables_rect.min.x
        );
        let left_margin = lane_rect.min.x;
        let right_margin = width - tables_rect.max.x;
        assert!(
            (left_margin - right_margin).abs() < 1.0,
            "{width}x{height}: left margin {left_margin:.1} vs right margin {right_margin:.1} stays centred"
        );
    }
}
