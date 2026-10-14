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
