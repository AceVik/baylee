use super::*;
use baylee_core::{
    ids::PlayerId,
    preset::{AIProfile, FormatId, GamePreset, SeatController},
};
use baylee_engine::{
    choice::{Pending, PlayerAction},
    engine::{Engine, NoLoopDetection},
    extensions::NativeLookup,
    zone::ZoneLocation,
};
use baylee_gamehost::{RegistryLookup, Session};
use baylee_protocol::execution::{Identity, Variant};

type Registry = NativeLookup<RegistryLookup, Pack>;
type Game = Engine<Registry, NoLoopDetection>;
type Hosted = Session<NoLoopDetection, Registry>;

mod life;

fn test_identity() -> Identity {
    Identity::native(
        Variant::Tower,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(v2::generation::CURRENT.ai_generation),
    )
}

fn preset(active: &[Definition]) -> GamePreset {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ))
    .unwrap();
    let a = baylee_cards::decks::load_acceptance(&text, "Allytifact").unwrap();
    let b = baylee_cards::decks::load_acceptance(&text, "Victory").unwrap();
    let mut preset = baylee_cards::decks::preset_for(19, &a, &b);
    preset.format = FormatId::Custom;
    let resources = [
        v2::loadout::resources(active, [1, 1], 0).unwrap(),
        v2::loadout::resources(active, [0, 0], 1).unwrap(),
    ];
    preset.house_rules = v2::loadout::rules(&resources, 17).unwrap();
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(v2::generation::CURRENT.expert());
    for (seat, resource) in preset.seats.iter_mut().zip(resources) {
        seat.starting_life = Some(resource.life);
        // Acceptance decks carry tournament sideboards; this Tower pool admits
        // only its purchased main deck, so no outside-game cards enter the fixture.
        seat.sideboard.clear();
        seat.commanders.clear();
    }
    v2::loadout::validate(&preset).unwrap();
    preset
}

#[test]
fn every_native_grove_rarity_uses_a_real_upkeep_trigger_and_ordinary_stack() {
    for rarity in Rarity::ALL {
        let definition = Definition {
            family: Family::Grove,
            rarity,
        };
        let mut game = Game::with_detection(&preset(&[definition]), Registry::default()).unwrap();
        assert_eq!(
            game.state().zones.list(ZoneLocation::Battlefield).len(),
            2,
            "setup creates artifacts, not their upkeep Plants"
        );
        for seat in [0, 1] {
            game.apply(PlayerId::new(seat), PlayerAction::MulliganKeep)
                .unwrap();
        }
        assert_eq!(game.state().zones.list(ZoneLocation::Stack).len(), 1);
        let active = game.state().turn.active;
        for _ in 0..8 {
            if game.state().zones.list(ZoneLocation::Stack).is_empty() {
                break;
            }
            let Pending::Priority { player, .. } = *game.pending() else {
                panic!("upkeep priority");
            };
            game.apply(player, PlayerAction::PassPriority).unwrap();
        }
        assert!(game.state().zones.list(ZoneLocation::Stack).is_empty());
        let plants: Vec<_> = game
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter_map(|id| {
                let object = game.state().arena.get(*id)?;
                object
                    .token
                    .is_some_and(|token| {
                        std::ptr::eq(
                            token,
                            &raw const baylee_cards::tokens::tower::TOWER_SEEDLING,
                        )
                    })
                    .then_some(object)
            })
            .collect();
        assert_eq!(plants.len(), usize::from(rarity.tier()));
        assert!(plants.iter().all(|plant| plant.base_controller == active));
        let Some(NativeDefinition::Permanent(token)) = Pack::definition(definition.id()) else {
            panic!("native Grove");
        };
        let id = baylee_cards::tokens::token_id(token);
        assert_ne!(id, u16::MAX);
        assert!(baylee_cards::tokens::ability_text(id, 0).is_some());
    }
}

#[test]
fn every_definition_is_admitted_and_resource_forgery_fails_in_the_actual_engine() {
    for family in Family::ALL {
        for rarity in Rarity::ALL {
            let preset = preset(&[Definition { family, rarity }]);
            assert!(Game::with_detection(&preset, Registry::default()).is_ok());
            let mut forged = preset.clone();
            forged.seats[0].starting_life = Some(999);
            assert!(Game::with_detection(&forged, Registry::default()).is_err());
            assert!(
                Engine::<RegistryLookup, NoLoopDetection>::with_detection(&preset, RegistryLookup)
                    .is_err()
            );
        }
    }
}

#[test]
fn native_tower_expert_play_resumes_and_replays_with_the_same_pack() {
    let preset = preset(&[Definition {
        family: Family::Grove,
        rarity: Rarity::Common,
    }]);
    let identity = test_identity();
    let mut live = Hosted::recorded_for_execution(&preset, &identity).unwrap();
    #[cfg(feature = "tower")]
    let _: &crate::variant::HostedSession = &live;
    let wrong = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Standard,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(AIProfile::GENERATION),
    );
    assert!(Hosted::recorded_for_execution(&preset, &wrong).is_none());
    let mut ordinary = preset.clone();
    ordinary.house_rules.rule_pack = None;
    assert!(Session::<NoLoopDetection>::recorded_for_execution(&ordinary, &identity).is_none());
    for _ in 0..32 {
        if live.needs_pump() {
            live.pump_slice(1);
        } else {
            let action = live.house_action(PlayerId::new(0)).unwrap();
            live.act_slice(PlayerId::new(0), action, 0).unwrap();
        }
    }
    let record = live.take_record();
    let resumed = Hosted::resume_with_detection(&record).unwrap();
    assert_eq!(resumed.snapshot_hash(), live.snapshot_hash());
    let replay =
        baylee_gamehost::record::replay_exact_with_registry::<NoLoopDetection, Registry>(&record)
            .unwrap();
    assert_eq!(replay.engine.snapshot_hash(), live.snapshot_hash());
}
