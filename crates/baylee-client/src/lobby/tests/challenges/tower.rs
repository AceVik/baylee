#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::tower::{Action as Tower, api as tower_api};

#[test]
fn tower_offers_three_reversible_previews_and_earned_tier_inspection_in_both_languages() {
    for lang in [Lang::En, Lang::De] {
        let (mut app, jobs) = screen(lang);
        {
            let mut state = app.world_mut().resource_mut::<LobbyState>();
            let model = &mut state.challenges.model;
            let task = model.detail.clone().unwrap();
            let words = || api::Words {
                en: "Tower".into(),
                de: "Turm".into(),
            };
            let season = tower_api::Season {
                id: task.id,
                withdrawn: false,
                theme: Box::new(tower_api::Theme {
                    period: api::Period::containing(api::Kind::Weekly, model.now).unwrap(),
                    title: words(),
                    floors: std::array::from_fn(|_| words()),
                    upgrades: std::array::from_fn(|_| {
                        std::array::from_fn(|_| tower_api::Upgrade {
                            title: words(),
                            summary: words(),
                            copies: 2,
                            remove: "Shock".into(),
                            add: "Lightning Bolt".into(),
                        })
                    }),
                }),
            };
            let mut progress = tower_api::Progress::new(true);
            progress.start().unwrap();
            progress
                .finish(api::AttemptState::Won, 9, Some(1000), true)
                .unwrap();
            model.selected = None;
            model.tower = baylee_client_core::tower::State::default();
            model.tower.opened = true;
            model.tower.loaded = true;
            model.tower.now = model.now;
            model.tower.season = Some(season.clone());
            model.tower.run = Some(tower_api::Run {
                id: task.id,
                season,
                revision: 2,
                progress,
                encounter: task,
                attempt: None,
                listed: false,
            });
            model.tower.badges = tower_api::BadgeKind::ALL
                .into_iter()
                .map(|kind| kind.progress(1))
                .collect();
        }
        settle(&mut app);
        for track in tower_api::Track::ALL {
            assert!(
                presses(&mut app)
                    .contains(&Press::Challenges(Action::Tower(Tower::Upgrade(track))))
            );
        }
        for _ in jobs.try_iter() {}
        press(
            &mut app,
            Press::Challenges(Action::Tower(Tower::Upgrade(tower_api::Track::Precision))),
        );
        settle(&mut app);
        assert!(presses(&mut app).contains(&Press::Challenges(Action::Tower(Tower::Choose))));
        assert!(
            jobs.try_recv().is_err(),
            "inspection never mutates the server"
        );
        press(&mut app, Press::Challenges(Action::Tower(Tower::Cancel)));
        settle(&mut app);
        assert!(
            app.world()
                .resource::<LobbyState>()
                .challenges
                .model
                .tower
                .upgrade
                .is_none()
        );
        press(
            &mut app,
            Press::Challenges(Action::Tower(Tower::Badge(tower_api::BadgeKind::Summit))),
        );
        settle(&mut app);
        assert!(labels(&mut app).contains(&Phrase::ChallengeObsidian.text(lang).to_string()));
        assert_eq!(
            app.world()
                .resource::<LobbyState>()
                .challenges
                .model
                .tower
                .badges[1]
                .count,
            1
        );
    }
}
