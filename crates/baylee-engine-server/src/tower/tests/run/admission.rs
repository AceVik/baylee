#[allow(clippy::wildcard_imports)]
use super::*;

#[cfg(feature = "tower")]
#[test]
fn compiled_executor_admits_all_five_published_starters_and_refuses_retained_setup() {
    use v2::package::{Build, Cohort, Origin, Package};
    let content = baylee_tower_content::standard().unwrap();
    let identity = crate::variant::identity();
    assert_eq!(identity.rules, "tower-6");
    assert_eq!(identity.ai, "tower-house-23");
    for starter in &content.starters {
        let build = Build {
            formation: starter.formation.clone(),
            deck: starter.deck.clone(),
            active: vec![item(Family::Grove, Rarity::Common)],
            potions: [1, 1],
        };
        let package = Package {
            schema: 2,
            cohort: Cohort::new(identity.clone(), &content.catalog).unwrap(),
            stage: 1,
            opponent: Origin::House { archetype: 1 },
            builds: [build.clone(), build],
            shuffle_seed: 19,
            policy_seed: 17,
        };
        let preset = package
            .preset(&content.catalog, &identity, |card| {
                Some(baylee_cards::decks::reference_print(card))
            })
            .unwrap();
        let live =
            crate::variant::HostedSession::recorded_for_execution(&preset, &identity).unwrap();
        let _: &baylee_gamehost::Session<NoLoopDetection, Registry> = &live;
        assert_eq!(preset.seats[0].deck.len(), 39);
        assert_eq!(preset.seats[0].commanders.len(), 1);
        assert!(
            live.state()
                .zones
                .list(ZoneLocation::Battlefield)
                .is_empty()
        );
        assert!(crate::tower::Pack::validate(&preset).is_err());
    }
    let retained = crate::tower::tests::preset(&[]);
    assert!(crate::variant::HostedSession::recorded_for_execution(&retained, &identity).is_none());
    assert!(crate::tower::Pack::validate(&retained).is_ok());
}

#[test]
fn all_catalog_tiers_are_independent_registered_sources_with_complete_action_text() {
    for family in Family::ALL {
        let mut previous_abilities = None;
        for rarity in Rarity::ALL {
            let definition = item(family, rarity);
            let preset = setup(&[definition], &[], 19);
            let game = Game::with_detection(&preset, Registry::default()).unwrap();
            let object = game
                .state()
                .arena
                .get(source(&game, PlayerId::new(0), definition))
                .unwrap();
            assert_eq!(object.kind, ObjectKind::Emblem);
            assert_eq!(object.zone, baylee_engine::zone::Zone::Command);
            assert!(
                game.state()
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .is_empty()
            );
            let token: &TokenDef = object.token.unwrap();
            if let Some(previous) = previous_abilities {
                assert_eq!(
                    token.abilities.len(),
                    previous + 1,
                    "each upgrade adds an ability: {family:?}"
                );
            }
            previous_abilities = Some(token.abilities.len());
            let id = baylee_cards::tokens::token_id(token);
            assert_ne!(id, u16::MAX);
            for index in 0..token.abilities.len() {
                assert!(
                    baylee_cards::tokens::ability_text(id, u32::try_from(index).unwrap()).is_some(),
                    "{family:?} {rarity:?} {index}"
                );
            }
            assert!(RunPack::definition(definition.id()).is_some());
            let mut forged = preset;
            forged.seats[0].deck[0].card = forged.seats[0].commanders[0].card;
            assert!(
                Game::with_detection(&forged, Registry::default()).is_err(),
                "native singleton check"
            );
        }
    }
    assert!(RunPack::definition(0).is_none());
    assert!(RunPack::definition(u16::try_from(Family::ALL.len() * 4 + 1).unwrap()).is_none());
}

#[test]
fn six_equipment_slots_apply_all_opening_bonuses_without_clamping_or_artifacts() {
    for family in [Family::Heartwood, Family::Satchel, Family::Compass] {
        let active = [item(family, Rarity::MythicRare); 6];
        let preset = setup(&active, &[], 19);
        let game = Game::with_detection(&preset, Registry::default()).unwrap();
        let expected = v2::catalog::run_attributes(&active).unwrap();
        assert_eq!(game.state().players[0].life, expected.life);
        assert_eq!(
            game.state()
                .zones
                .list(ZoneLocation::Hand(PlayerId::new(0)))
                .len(),
            usize::from(expected.cards)
        );
        assert_eq!(
            preset.house_rules.opening[0].free_mulligans,
            expected.free_mulligans
        );
        assert_eq!(
            preset.house_rules.opening[0].maximum_hand,
            expected.maximum_hand
        );
        assert!(
            game.state()
                .zones
                .list(ZoneLocation::Battlefield)
                .is_empty()
        );
        assert_eq!(
            game.state()
                .zones
                .list(ZoneLocation::Command(PlayerId::new(0)))
                .len(),
            9,
            "six items, commander, fixed private potion sources"
        );
    }
}

#[test]
fn independent_grove_and_chalice_really_trigger_and_keep_their_sources() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let grove = item(Family::Grove, rarity);
        let mut game = prepared(&[grove], &[], 19);
        let id = source(&game, player, grove);
        main(&mut game, 1, player);
        assert_eq!(plants(&game, player), usize::from(rarity.tier()));
        let gained = if rarity == Rarity::Common {
            0
        } else {
            i32::from(rarity.tier())
        };
        assert_eq!(game.state().players[0].life, 20 + gained);
        assert_eq!(
            game.state().arena.get(id).unwrap().zone,
            baylee_engine::zone::Zone::Command
        );
        let chalice = item(Family::SpringChalice, rarity);
        let mut game = prepared(&[chalice], &[], 19);
        main(&mut game, 1, player);
        let tier = usize::from(rarity.tier() - 1);
        let ordinary_draw = usize::from(game.state().turn.number > 1);
        let draw_life = if tier == 0 {
            0
        } else {
            ordinary_draw + usize::from(tier == 3)
        };
        assert_eq!(
            game.state().players[0].life,
            20 + [1, 2, 3, 5][tier] + i32::try_from(draw_life).unwrap()
        );
    }
}
