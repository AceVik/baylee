//! Independent Drain Power, Illusionary Mask and Word of Command card behavior.
//! Production authors separately test receipts, constrained payments and privacy.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_core::ids::DamageSourceRef;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority {player: holder, ..} if *holder == player),
    );
}

fn tap_card(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: CardIndex) {
    let source = all_on_battlefield(engine, player, card)
        .into_iter()
        .find(|id| !is_tapped(engine, *id))
        .unwrap();
    engine
        .apply(player, PlayerAction::ActivateManaAbility { source })
        .unwrap();
}

fn drain_fixture() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest(), sol_ring()])
        .hand(0, &[index::DRAIN_POWER])
        .battlefield(1, &[forest(), mountain(), swamp(), sol_ring()])
        .hand(1, &[index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

fn announce_drain(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    priority(engine, USER);
    tap_card(engine, USER, island());
    tap_card(engine, USER, island());
    cast_with_floating(engine, USER, index::DRAIN_POWER);
    engine
        .apply(USER, PlayerAction::ChoosePlayer(target))
        .unwrap();
}

fn finish_drain(engine: &mut Engine<RegistryLookup>, affected: PlayerId) -> Vec<DamageSourceRef> {
    let mut activated = Vec::new();
    for _ in 0..50 {
        if stack_is_empty(engine) {
            return activated;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseManaAbility {
                player,
                choice,
                options,
                ..
            } => {
                assert_eq!(
                    player, affected,
                    "land controller chooses, not Drain's caster"
                );
                assert!(!options.is_empty());
                for option in &options {
                    let object = engine.state().object(option.source.object).unwrap();
                    assert_eq!(object.controller, affected);
                    assert!(object.characteristics().types.contains(TypeSet::LAND));
                }
                let option = options[0];
                assert!(
                    !activated.contains(&option.source),
                    "only one activation per listed land"
                );
                activated.push(option.source);
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseManaAbility {
                            choice,
                            source: option.source,
                            ability_index: option.ability_index,
                        },
                    )
                    .unwrap();
            }
            pending => panic!("unexpected Drain Power choice: {pending:?}"),
        }
    }
    panic!("Drain Power did not finish");
}

#[test]
fn drain_review_forces_untapped_lands_then_transfers_preexisting_mana_too() {
    let mut engine = drain_fixture();
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, swamp());
    let black_land = on_battlefield(&engine, OTHER, swamp()).unwrap();
    announce_drain(&mut engine, OTHER);
    let activated = finish_drain(&mut engine, OTHER);
    assert_eq!(activated.len(), 2);
    assert!(activated.iter().all(|r| r.object != black_land));
    for color in [ManaColor::Black, ManaColor::Green, ManaColor::Red] {
        assert_eq!(engine.state().players[0].mana_pool.available(color), 1);
        assert_eq!(engine.state().players[1].mana_pool.available(color), 0);
    }
    assert!(!is_tapped(
        &engine,
        on_battlefield(&engine, OTHER, sol_ring()).unwrap()
    ));
    assert!(!is_tapped(
        &engine,
        on_battlefield(&engine, USER, forest()).unwrap()
    ));
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn drain_review_can_target_its_caster_without_duplicating_the_pool() {
    let mut engine = drain_fixture();
    announce_drain(&mut engine, USER);
    let activated = finish_drain(&mut engine, USER);
    assert_eq!(activated.len(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0
    );
    assert!(!is_tapped(
        &engine,
        on_battlefield(&engine, USER, sol_ring()).unwrap()
    ));
    assert!(!is_tapped(
        &engine,
        on_battlefield(&engine, OTHER, forest()).unwrap()
    ));
}

#[test]
fn drain_review_opponent_may_cast_dark_ritual_before_the_mana_instruction() {
    let mut engine = drain_fixture();
    announce_drain(&mut engine, OTHER);
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, swamp());
    cast_with_floating(&mut engine, OTHER, index::DARK_RITUAL);
    let activated = finish_drain(&mut engine, OTHER);
    assert_eq!(activated.len(), 2);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, OTHER, index::DARK_RITUAL).is_some());
}

fn mask_fixture(
    mana_land: CardIndex,
    mana_count: usize,
    hand: &[CardIndex],
    vigilance: bool,
) -> Engine<RegistryLookup> {
    let mut board = vec![mana_land; mana_count];
    board.push(index::ILLUSIONARY_MASK);
    if vigilance {
        board.push(index::SERRA_S_BLESSING);
    }
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .battlefield(
            1,
            &[
                plains(),
                plains(),
                mountain(),
                forest(),
                index::ICY_MANIPULATOR,
                llanowar_elves(),
            ],
        )
        .hand(
            1,
            &[
                index::DISENCHANT,
                lightning_bolt(),
                counterspell(),
                index::HEALING_SALVE,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

fn mask_select(
    engine: &mut Engine<RegistryLookup>,
    land: CardIndex,
    x: u32,
    card: CardIndex,
) -> (ObjectId, Vec<ObjectId>) {
    for _ in 0..x {
        tap_card(engine, USER, land);
    }
    activate(engine, USER, index::ILLUSIONARY_MASK, 0);
    engine.apply(USER, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let selected = in_hand(engine, USER, card).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(options.contains(&selected));
    let offered = options.clone();
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![selected],
            },
        )
        .unwrap();
    (selected, offered)
}

#[test]
fn mask_review_actual_blue_payment_allows_storm_crow_but_not_grizzly_bears() {
    let mut engine = mask_fixture(
        island(),
        2,
        &[index::STORM_CROW, index::GRIZZLY_BEARS],
        false,
    );
    let bear = in_hand(&engine, USER, index::GRIZZLY_BEARS).unwrap();
    let (crow, options) = mask_select(&mut engine, island(), 2, index::STORM_CROW);
    assert!(
        !options.contains(&bear),
        "UU cannot pay the Bears’ green symbol"
    );
    // Only the selected face-down spell is cast, and the wrong-color card stays in hand.
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().object(crow).unwrap().zone, Zone::Stack);
    assert!(
        engine
            .state()
            .object(crow)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, crow), (2, 2));
    assert!(!keywords(&engine, crow).contains(KeywordSet::FLYING));
}

#[test]
fn mask_review_tap_reveals_after_mask_leaves_without_fire_imps_enter_trigger() {
    let mut engine = mask_fixture(mountain(), 3, &[index::FIRE_IMP], false);
    let (imp, _) = mask_select(&mut engine, mountain(), 3, index::FIRE_IMP);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, imp), (2, 2));
    let elf = on_battlefield(&engine, OTHER, llanowar_elves()).unwrap();
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    let mask = on_battlefield(&engine, USER, index::ILLUSIONARY_MASK).unwrap();
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, plains());
    tap_card(&mut engine, OTHER, plains());
    cast_with_floating(&mut engine, OTHER, index::DISENCHANT);
    aim(&mut engine, vec![mask], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, USER, index::ILLUSIONARY_MASK).is_some());
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, forest());
    activate(&mut engine, OTHER, index::ICY_MANIPULATOR, 0);
    aim(&mut engine, vec![imp], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, imp), (2, 1));
    assert!(is_tapped(&engine, imp));
    assert!(
        !engine
            .state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(
        engine.state().object(elf).unwrap().damage,
        0,
        "neither entry face down nor turning face up triggers Fire Imp"
    );
}

#[test]
fn mask_review_receiving_bolt_reveals_dragon_before_lethal_damage_is_checked() {
    let mut engine = mask_fixture(mountain(), 6, &[index::SHIVAN_DRAGON], false);
    let (dragon, _) = mask_select(&mut engine, mountain(), 6, index::SHIVAN_DRAGON);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (2, 2));
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, mountain());
    cast_with_floating(&mut engine, OTHER, lightning_bolt());
    aim(&mut engine, vec![dragon], vec![]);
    assert!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "being targeted alone does not turn the creature face up"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert_eq!(
        engine.state().object(dragon).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(dragon).unwrap().damage, 3);
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
}

#[test]
fn mask_review_vigilant_attacker_reveals_before_assigning_five_combat_damage() {
    let mut engine = mask_fixture(mountain(), 6, &[index::SHIVAN_DRAGON], true);
    let (dragon, _) = mask_select(&mut engine, mountain(), 6, index::SHIVAN_DRAGON);
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, OTHER);
    reach_their_main_phase(&mut engine, USER);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            USER,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(OTHER))],
            },
        )
        .unwrap();
    assert!(
        !is_tapped(&engine, dragon),
        "granted vigilance avoids the earlier tap replacement"
    );
    assert!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(OTHER, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert!(
        !engine
            .state()
            .object(dragon)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
}

const THIRD: PlayerId = PlayerId::new(2);

fn word_fixture(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[index::WORD_OF_COMMAND])
        .battlefield(1, board)
        .hand(1, hand)
        .battlefield(2, &[forest()])
        .hand(2, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

fn word_select(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    priority(engine, USER);
    tap_card(engine, USER, swamp());
    tap_card(engine, USER, swamp());
    cast_with_floating(engine, USER, index::WORD_OF_COMMAND);
    match engine.pending() {
        Pending::ChoosePlayer { .. } => {
            engine
                .apply(USER, PlayerAction::ChoosePlayer(OTHER))
                .unwrap();
        }
        Pending::ChooseTargets { .. } => aim(engine, vec![], vec![OTHER]),
        pending => panic!("Word opponent target expected: {pending:?}"),
    }
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let selected = in_hand(engine, OTHER, card).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(options.contains(&selected));
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![selected],
            },
        )
        .unwrap();
    selected
}

fn finish_word_mana(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..20 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            return;
        };
        if engine.decision_actor() != Some(USER) || player != OTHER {
            return;
        }
        assert_eq!(engine.payment_window().unwrap().0, OTHER);
        assert!(legal.granted_actions.is_empty());
        if let Some(source) = legal.mana_abilities.first() {
            assert_eq!(engine.state().object(*source).unwrap().controller, OTHER);
            assert!(
                engine
                    .state()
                    .object(*source)
                    .unwrap()
                    .characteristics()
                    .types
                    .contains(TypeSet::LAND)
            );
            engine
                .apply(USER, PlayerAction::ActivateManaAbility { source: *source })
                .unwrap();
        } else {
            engine.apply(USER, PlayerAction::PassPriority).unwrap();
        }
    }
    panic!("Word mana payment did not finish");
}

#[test]
fn word_review_actor_pays_opponent_resources_and_third_seat_gains_no_private_entitlement() {
    let mut engine = word_fixture(&[mountain()], &[lightning_bolt()]);
    let bolt = word_select(&mut engine, lightning_bolt());
    let Pending::ChooseTargets { player, .. } = engine.pending() else {
        panic!("forced Bolt targets expected")
    };
    assert_eq!(*player, OTHER);
    assert_eq!(engine.decision_actor(), Some(USER));
    assert!(engine.may_inspect_private(USER, OTHER));
    assert!(!engine.may_inspect_private(THIRD, OTHER));
    assert!(!engine.may_inspect_private(USER, THIRD));
    engine
        .apply(
            USER,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![OTHER],
            },
        )
        .unwrap();
    finish_word_mana(&mut engine);
    assert_eq!(engine.state().object(bolt).unwrap().controller, OTHER);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "only BB was spent on Word itself"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(is_tapped(
        &engine,
        on_battlefield(&engine, OTHER, mountain()).unwrap()
    ));
    assert!(in_graveyard(&engine, USER, index::WORD_OF_COMMAND).is_some());
    priority(&mut engine, OTHER);
    assert_eq!(
        engine.decision_actor(),
        Some(OTHER),
        "ordinary responses belong to the responding player"
    );
    assert!(!engine.may_inspect_private(USER, OTHER));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[2].life, 20);
}

#[test]
fn word_review_selected_tutor_resolution_restores_actor_but_keeps_resource_owner() {
    let mut engine = word_fixture(&[swamp(), swamp()], &[index::DEMONIC_TUTOR]);
    word_select(&mut engine, index::DEMONIC_TUTOR);
    finish_word_mana(&mut engine);
    priority(&mut engine, OTHER);
    assert_eq!(engine.decision_actor(), Some(OTHER));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, OTHER);
    assert_eq!(engine.decision_actor(), Some(USER));
    assert!(!engine.may_inspect_private(THIRD, OTHER));
    let card = options[0];
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(card).unwrap().zone, Zone::Hand);
    assert_eq!(engine.state().object(card).unwrap().owner, OTHER);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(OTHER))
            .contains(&card)
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(USER))
            .contains(&card)
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!engine.may_inspect_private(USER, OTHER));
}

#[test]
fn word_review_sol_rings_cannot_pay_a_commanded_generic_cost() {
    let mut engine = word_fixture(&[sol_ring(), sol_ring()], &[index::JUGGERNAUT]);
    let juggernaut = word_select(&mut engine, index::JUGGERNAUT);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(juggernaut).unwrap().zone, Zone::Hand);
    for ring in all_on_battlefield(&engine, OTHER, sol_ring()) {
        assert!(!is_tapped(&engine, ring));
    }
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn word_review_land_play_obeys_opponents_turn_and_used_land_allowance() {
    for (opponents_turn, already_played) in [(false, false), (true, false), (true, true)] {
        let mut engine = word_fixture(&[mountain()], &[forest(), plains()]);
        if opponents_turn {
            reach_their_main_phase(&mut engine, OTHER);
        }
        if already_played {
            let land = in_hand(&engine, OTHER, plains()).unwrap();
            engine
                .apply(OTHER, PlayerAction::PlayLand { card: land })
                .unwrap();
        }
        let forest = word_select(&mut engine, forest());
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().object(forest).unwrap().zone,
            if opponents_turn && !already_played {
                Zone::Battlefield
            } else {
                Zone::Hand
            }
        );
        assert_eq!(engine.state().object(forest).unwrap().owner, OTHER);
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(engine.state().players[1].life, 20);
    }
}

#[test]
fn mask_review_may_decline_casting_but_cannot_activate_on_another_players_turn() {
    let mut engine = mask_fixture(mountain(), 3, &[index::FIRE_IMP], false);
    let imp = in_hand(&engine, USER, index::FIRE_IMP).unwrap();
    for _ in 0..3 {
        tap_card(&mut engine, USER, mountain());
    }
    activate(&mut engine, USER, index::ILLUSIONARY_MASK, 0);
    engine.apply(USER, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(USER, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(imp).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "declining does not refund the activation"
    );
    reach_their_main_phase(&mut engine, OTHER);
    priority(&mut engine, USER);
    let mask = on_battlefield(&engine, USER, index::ILLUSIONARY_MASK).unwrap();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: mask,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(engine.state().object(imp).unwrap().zone, Zone::Hand);
}

#[test]
fn word_review_forced_force_of_will_pitches_opponents_card_and_pays_opponents_life() {
    let mut engine = word_fixture(
        &[mountain()],
        &[lightning_bolt(), index::FORCE_OF_WILL, index::STORM_CROW],
    );
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, mountain());
    cast_with_floating(&mut engine, OTHER, lightning_bolt());
    aim(&mut engine, vec![], vec![USER]);
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    let pitch = in_hand(&engine, OTHER, index::STORM_CROW).unwrap();
    let force = word_select(&mut engine, index::FORCE_OF_WILL);
    for _ in 0..10 {
        if in_graveyard(&engine, USER, index::WORD_OF_COMMAND).is_some() {
            break;
        }
        assert_eq!(engine.decision_actor(), Some(USER));
        match engine.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                assert_eq!(player, OTHER);
                let mode = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .unwrap();
                engine.apply(USER, PlayerAction::ChooseMode(mode)).unwrap();
            }
            Pending::ChooseTargets { player, .. } => {
                assert_eq!(player, OTHER);
                engine
                    .apply(
                        USER,
                        PlayerAction::ChooseTargets {
                            objects: vec![bolt],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, OTHER);
                assert_eq!((min, max), (1, 1));
                assert!(options.contains(&pitch));
                engine
                    .apply(
                        USER,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .unwrap();
            }
            pending => panic!("unexpected forced pitch choice: {pending:?}"),
        }
    }
    assert!(in_graveyard(&engine, USER, index::WORD_OF_COMMAND).is_some());
    assert_eq!(engine.state().object(force).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().object(force).unwrap().controller, OTHER);
    assert_eq!(engine.state().object(pitch).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(pitch).unwrap().owner, OTHER);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the commanded Force counters the original Bolt"
    );
    assert!(in_graveyard(&engine, OTHER, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, OTHER, index::FORCE_OF_WILL).is_some());
}

#[test]
fn word_review_overproducing_land_refuses_without_tapping_then_exact_payment_succeeds() {
    let mut engine = word_fixture(
        &[plains(), plains(), index::AZORIUS_CHANCERY],
        &[index::WHITE_KNIGHT],
    );
    reach_their_main_phase(&mut engine, OTHER);
    let knight = word_select(&mut engine, index::WHITE_KNIGHT);
    let chancery = on_battlefield(&engine, OTHER, index::AZORIUS_CHANCERY).unwrap();
    assert_eq!(
        engine.payment_window().map(|(player, _)| player),
        Some(OTHER)
    );
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: chancery,
                    ability_index: 0,
                }
            )
            .is_err(),
        "the surplus blue has no permitted consumer"
    );
    assert_eq!(engine.fingerprint(), before);
    assert!(!is_tapped(&engine, chancery));
    finish_word_mana(&mut engine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(knight).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(knight).unwrap().controller, OTHER);
    assert_eq!(pt(&engine, knight), (2, 2));
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, chancery));
}

#[test]
fn mask_review_noncombat_prevention_order_controls_whether_recipient_turns_face_up() {
    for prevent_first in [true, false] {
        let mut engine = mask_fixture(mountain(), 6, &[index::SHIVAN_DRAGON], false);
        let (dragon, _) = mask_select(&mut engine, mountain(), 6, index::SHIVAN_DRAGON);
        pass_until(&mut engine, stack_is_empty);
        priority(&mut engine, OTHER);
        tap_card(&mut engine, OTHER, mountain());
        cast_with_floating(&mut engine, OTHER, lightning_bolt());
        aim(&mut engine, vec![dragon], vec![]);
        priority(&mut engine, OTHER);
        tap_card(&mut engine, OTHER, plains());
        cast_with_floating(&mut engine, OTHER, index::HEALING_SALVE);
        let Pending::ChooseCastMode { options, .. } = engine.pending() else {
            panic!("Salve modes expected");
        };
        let prevention = options
            .iter()
            .position(|option| option.kind == CastModeKind::Mode(1))
            .unwrap();
        engine
            .apply(OTHER, PlayerAction::ChooseMode(prevention))
            .unwrap();
        aim(&mut engine, vec![dragon], vec![]);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseDamageEffect { .. })
        });
        let Pending::ChooseDamageEffect {
            player,
            choice,
            options,
            ..
        } = engine.pending()
        else {
            unreachable!()
        };
        assert_eq!(
            *player, USER,
            "the recipient's controller chooses, not Salve's caster"
        );
        let effect = options
            .iter()
            .find(|option| {
                if prevent_first {
                    matches!(
                        option.kind,
                        crate::choice::DamageEffectKind::PreventNext { .. }
                    )
                } else {
                    matches!(
                        option.kind,
                        crate::choice::DamageEffectKind::TurnFaceUp { .. }
                    )
                }
            })
            .unwrap()
            .id;
        engine
            .apply(
                USER,
                PlayerAction::ChooseDamageEffect {
                    choice: *choice,
                    effect,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().object(dragon).unwrap().damage, 0);
        assert_eq!(
            pt(&engine, dragon),
            if prevent_first { (2, 2) } else { (5, 5) }
        );
        assert_eq!(
            engine
                .state()
                .object(dragon)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN),
            prevent_first
        );
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(engine.state().players[1].life, 20);
    }
}

fn word_channel_fixture() -> (Engine<RegistryLookup>, baylee_core::ids::GrantedActionId) {
    use crate::choice::GrantedActionKind;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[index::WORD_OF_COMMAND])
        .battlefield(1, &[forest(), forest(), plains()])
        .hand(
            1,
            &[index::CHANNEL, index::GUARDIAN_ANGEL, index::JUGGERNAUT],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    reach_their_main_phase(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, forest());
    tap_card(&mut engine, OTHER, forest());
    cast_with_floating(&mut engine, OTHER, index::CHANNEL);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, OTHER);
    tap_card(&mut engine, OTHER, plains());
    cast_with_floating(&mut engine, OTHER, index::GUARDIAN_ANGEL);
    engine.apply(OTHER, PlayerAction::ChooseNumber(0)).unwrap();
    aim(&mut engine, vec![], vec![OTHER]);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, OTHER);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    let guardian = legal
        .granted_actions
        .iter()
        .find(|offer| matches!(offer.effect, GrantedActionKind::PreventNextDamage { .. }))
        .expect("Guardian is available at ordinary priority")
        .id;
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    (engine, guardian)
}

#[test]
fn word_review_channel_special_action_pays_commanded_spell_and_keeps_surplus() {
    use crate::choice::GrantedActionKind;
    let (mut engine, guardian) = word_channel_fixture();
    let juggernaut = word_select(&mut engine, index::JUGGERNAUT);
    assert_eq!(
        engine.payment_window().map(|(player, _)| player),
        Some(OTHER)
    );
    assert_eq!(engine.decision_actor(), Some(USER));
    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("payment");
    };
    assert_eq!(*player, OTHER);
    assert!(
        !legal
            .granted_actions
            .iter()
            .any(|offer| offer.id == guardian)
    );
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: guardian })
            .is_err()
    );
    assert_eq!(
        engine.fingerprint(),
        before,
        "priority-only action refuses atomically"
    );
    for paid in 1..=5 {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("payment");
        };
        let channel = legal
            .granted_actions
            .iter()
            .find(|offer| {
                matches!(
                    offer.effect,
                    GrantedActionKind::AddMana {
                        color: ManaColor::Colorless,
                        amount: 1
                    }
                )
            })
            .expect("Channel's mana-timed special action remains available")
            .id;
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: channel })
            .unwrap();
        assert_eq!(engine.state().players[1].life, 20 - paid);
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(
            engine.payment_window().map(|(player, _)| player),
            Some(OTHER)
        );
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().object(juggernaut).unwrap().zone, Zone::Stack);
    assert!(in_graveyard(&engine, USER, index::WORD_OF_COMMAND).is_some());
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "Channel units are not bound land-ability production"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(juggernaut).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(juggernaut).unwrap().controller, OTHER);
    assert_eq!(pt(&engine, juggernaut), (5, 3));
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
}
