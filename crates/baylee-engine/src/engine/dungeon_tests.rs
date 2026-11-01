//! Immediate dungeon plays exercise the normal casting wizard during resolution.
use super::testkit::*;
use super::*;
use crate::object::ObjectKind;
use crate::resolve::{Flow, Resolution};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::Effect;
use baylee_core::ids::SeatSet;

fn resolution(spell: ObjectId, effects: &[Effect]) -> Resolution {
    Resolution {
        source: spell,
        on_stack: spell,
        controller: PlayerId::new(0),
        effects: effects.to_vec(),
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

fn start_room(engine: &mut Engine<RegistryLookup>, effect: Effect) {
    let player = PlayerId::new(0);
    let name = engine.state.names.intern("Dungeon room");
    let source = engine.state.create_bare(
        player,
        ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    let mut res = resolution(source, &[effect]);
    match crate::resolve::run(&mut engine.state, &mut res) {
        Flow::Wait(pending) => {
            engine.resolution = Some(res);
            engine.pending = pending;
            engine.awaiting_answer = true;
            engine.run_until_choice();
        }
        _ => panic!("room should offer a play"),
    }
}

#[test]
fn mad_wizards_lair_draws_three_then_casts_exactly_one_spell_for_free() {
    let p0 = PlayerId::new(0);
    let ring_card = card_index("6ad8011d-3471-4369-9d68-b264cc027487");
    let mut engine = Duel::new(718, basic_forest()).hand(0, &[ring_card]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = in_hand(&engine, p0, ring_card).unwrap();
    let _ = engine.state.move_object(
        ring,
        ZoneLocation::Library(p0),
        ZonePosition::Top,
        Cause::Effect,
    );
    let before = engine.state.zones.list(ZoneLocation::Hand(p0)).len();
    start_room(&mut engine, Effect::DungeonDrawThree);
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(p0)).len(),
        before + 3
    );
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("choose drawn spell")
    };
    assert_eq!(options, &[ring], "lands are revealed but cannot be cast");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    assert_eq!(engine.state.object(ring).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state.players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state.object(ring).unwrap().zone, Zone::Battlefield);
}

#[test]
fn runestone_caverns_can_play_only_one_land_and_leaves_the_other_exiled() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(719, basic_forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    start_room(&mut engine, Effect::DungeonExileTwo);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("exiled cards")
    };
    assert_eq!(options.len(), 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state.object(options[0]).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state.object(options[1]).unwrap().zone, Zone::Exile);
    assert!(
        !engine
            .state
            .object(options[1])
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(_)))
    );
    assert!(stack_is_empty(&engine));
}

#[test]
fn merged_commander_redirects_only_its_own_physical_card() {
    use crate::object::Rider;
    use baylee_core::generated::index;
    for commander_on_root in [false, true] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(720, basic_forest())
            .commander(0, &[index::GEMRAZER])
            .battlefield(0, &[index::LLANOWAR_ELVES])
            .start();
        keep_mulligans(&mut engine);
        let commander = engine.state.commanders[0][0].object;
        let elf = on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap();
        let (root, part) = if commander_on_root {
            engine.state.object_mut(commander).unwrap().kind = ObjectKind::Permanent;
            engine
                .state
                .move_object(
                    commander,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            engine
                .state
                .move_object(
                    elf,
                    ZoneLocation::Command(p0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            (commander, elf)
        } else {
            (elf, commander)
        };
        engine
            .state
            .object_mut(root)
            .unwrap()
            .riders
            .push(Rider::MergedComponent(part));
        engine
            .state
            .object_mut(part)
            .unwrap()
            .riders
            .push(Rider::MergedInto(root));
        let mut res = resolution(
            ObjectId::NO_SOURCE,
            &[Effect::ReturnToHand {
                target: baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
            }],
        );
        res.targets.push(root);
        let Flow::Wait(Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::CommanderReplace { card, .. },
            ..
        }) = crate::resolve::run(&mut engine.state, &mut res)
        else {
            panic!("physical commander gets its choice")
        };
        assert_eq!(card, commander);
        assert!(matches!(
            crate::resolve::resume_yes_no(&mut engine.state, &mut res, true),
            Flow::Complete
        ));
        assert_eq!(engine.state.object(commander).unwrap().zone, Zone::Command);
        assert_eq!(engine.state.object(elf).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn countering_the_last_room_still_completes_the_dungeon() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(721, basic_forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let name = engine.state.names.intern("Lost Mine of Phandelver");
    let dungeon = engine
        .state
        .create_bare(p0, ObjectKind::Emblem, name, ZoneLocation::Command(p0));
    engine.state.players[0].dungeon = Some((baylee_core::dungeon::Dungeon::LostMine, dungeon));
    engine.state.players[0].dungeon_room = 7;
    let room = engine
        .state
        .create_bare(p0, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
    engine.state.object_mut(room).unwrap().ability = Some(crate::object::AbilityLoc {
        card: None,
        index: baylee_core::ids::AbilityRef::SYNTHETIC,
        source: dungeon,
    });
    let mut counter = resolution(ObjectId::NO_SOURCE, &[Effect::CounterTargetAbility]);
    counter.targets.push(room);
    assert!(matches!(
        crate::resolve::run(&mut engine.state, &mut counter),
        Flow::Complete
    ));
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(engine.state.players[0].dungeon.is_none());
    assert_eq!(engine.state.players[0].dungeons_completed, 1);
    assert!(engine.state.object(dungeon).is_none());
}

#[test]
fn runestone_rechecks_castability_after_playing_the_other_exiled_land() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(722, basic_forest())
        .hand(0, &[index::SOL_RING])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = in_hand(&engine, p0, index::SOL_RING).unwrap();
    engine
        .state
        .move_object(
            ring,
            ZoneLocation::Library(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    start_room(&mut engine, Effect::DungeonExileTwo);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("only the land can currently be played")
    };
    assert_eq!(options.len(), 1);
    assert!(!options.contains(&ring));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ring]
                }
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("the newly played Forest can now pay for Sol Ring")
    };
    assert_eq!(options, &[ring]);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(stack_is_empty(&engine));
}

#[test]
fn exiling_a_merged_creature_links_each_physical_card_and_blink_returns_both() {
    use crate::object::Rider;
    use baylee_core::generated::index;
    for linked in [false, true] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(723, basic_forest())
            .battlefield(0, &[index::LLANOWAR_ELVES, index::GEMRAZER])
            .start();
        keep_mulligans(&mut engine);
        let root = on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap();
        let part = on_battlefield(&engine, p0, index::GEMRAZER).unwrap();
        engine
            .state
            .move_object(
                part,
                ZoneLocation::Command(p0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        engine
            .state
            .object_mut(root)
            .unwrap()
            .riders
            .push(Rider::MergedComponent(part));
        engine
            .state
            .object_mut(part)
            .unwrap()
            .riders
            .push(Rider::MergedInto(root));
        let effect = if linked {
            Effect::ExileTargetsWithSource
        } else {
            Effect::Blink {
                target: baylee_cards_dsl::TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
                owner_control: true,
            }
        };
        let mut res = resolution(ObjectId::NO_SOURCE, &[effect]);
        res.targets.push(root);
        assert!(matches!(
            crate::resolve::run(&mut engine.state, &mut res),
            Flow::Complete
        ));
        for id in [root, part] {
            let card = engine.state.object(id).unwrap();
            assert_eq!(
                card.zone,
                if linked {
                    Zone::Exile
                } else {
                    Zone::Battlefield
                }
            );
            assert!(!card.is_merged_component());
            if linked {
                assert!(
                    card.riders
                        .iter()
                        .any(|r| matches!(r, Rider::LinkedExile { .. }))
                );
                assert!(
                    card.riders
                        .iter()
                        .any(|r| matches!(r, Rider::ExiledWith { .. }))
                );
            }
        }
    }
}
