//! Independent text/copy card acceptance against cached Oracle and CR 400.7,
//! 612.2–4 and 707.9a–d. Production authors own generic mapping/provenance tests.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority {player: holder, ..} if *holder == player),
    );
}

fn copy_fixture(first: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut board = vec![island(); 8];
    board.extend([
        plains(),
        llanowar_elves(),
        index::GRIZZLY_BEARS,
        index::WALL_OF_SWORDS,
        index::SHIVAN_DRAGON,
    ]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(
            0,
            &[index::VESUVAN_DOPPELGANGER, index::CLONE, ephemerate()],
        )
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let target = on_battlefield(&engine, USER, first).unwrap();
    cast_from_hand(&mut engine, USER, index::VESUVAN_DOPPELGANGER);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![target], vec![]);
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, USER, index::VESUVAN_DOPPELGANGER).unwrap();
    assert_eq!(
        engine
            .state()
            .object(copy)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Blue)
    );
    (engine, copy)
}

fn next_upkeep(engine: &mut Engine<RegistryLookup>) {
    reach_their_main_phase(engine, OTHER);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(engine.state().turn.active, USER);
    assert_eq!(engine.state().turn.step, crate::turn::Step::Upkeep);
}

fn finish_copies(engine: &mut Engine<RegistryLookup>, target: ObjectId, accept: bool) -> usize {
    let mut targets = 0;
    let mut started = false;
    for _ in 0..60 {
        if started && stack_is_empty(engine) {
            return targets;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                aim(engine, vec![target], vec![]);
                targets += 1;
                started = true;
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                engine.apply(player, PlayerAction::YesNo(accept)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            pending => panic!("unexpected copy prompt: {pending:?}"),
        }
    }
    panic!("copy triggers did not finish");
}

#[test]
fn vesuvan_review_upkeep_copy_can_be_declined_then_changed_without_losing_color() {
    let (mut engine, copy) = copy_fixture(llanowar_elves());
    let bear = on_battlefield(&engine, USER, index::GRIZZLY_BEARS).unwrap();
    next_upkeep(&mut engine);
    assert_eq!(finish_copies(&mut engine, bear, false), 1);
    assert_eq!(pt(&engine, copy), (1, 1));
    next_upkeep(&mut engine);
    assert_eq!(finish_copies(&mut engine, bear, true), 1);
    assert_eq!(pt(&engine, copy), (2, 2));
    assert_eq!(
        engine
            .state()
            .object(copy)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::Blue)
    );
}

#[test]
fn vesuvan_review_self_copy_creates_two_real_upkeep_triggers() {
    let (mut engine, copy) = copy_fixture(llanowar_elves());
    next_upkeep(&mut engine);
    assert_eq!(finish_copies(&mut engine, copy, true), 1);
    next_upkeep(&mut engine);
    let bear = on_battlefield(&engine, USER, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(
        finish_copies(&mut engine, bear, false),
        2,
        "both copied instances trigger separately"
    );
    assert_eq!(pt(&engine, copy), (1, 1));
}

#[test]
fn vesuvan_review_clone_inherits_the_copiable_recurring_ability() {
    let (mut engine, copy) = copy_fixture(llanowar_elves());
    priority(&mut engine, USER);
    cast_from_hand(&mut engine, USER, index::CLONE);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![copy], vec![]);
    pass_until(&mut engine, stack_is_empty);
    let clone = on_battlefield(&engine, USER, index::CLONE).unwrap();
    assert_eq!(pt(&engine, clone), (1, 1));
    next_upkeep(&mut engine);
    let wall = on_battlefield(&engine, USER, index::WALL_OF_SWORDS).unwrap();
    assert_eq!(finish_copies(&mut engine, wall, true), 2);
    for object in [copy, clone] {
        assert_eq!(pt(&engine, object), (3, 5));
        assert_eq!(
            engine
                .state()
                .object(object)
                .unwrap()
                .characteristics()
                .colors,
            ColorSet::of(Color::Blue)
        );
    }
}

#[test]
fn vesuvan_review_changing_form_preserves_real_damage_and_does_not_enter_again() {
    let (mut engine, copy) = copy_fixture(index::WALL_OF_SWORDS);
    let dragon = on_battlefield(&engine, USER, index::SHIVAN_DRAGON).unwrap();
    let version = engine.state().object(copy).unwrap().version;
    next_upkeep(&mut engine);
    aim(&mut engine, vec![dragon], vec![]);
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, lightning_bolt());
    aim(&mut engine, vec![copy], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().object(copy).unwrap().damage, 3);
    engine.apply(USER, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, copy), (5, 5));
    assert_eq!(engine.state().object(copy).unwrap().damage, 3);
    assert_eq!(engine.state().object(copy).unwrap().version, version);
}

#[test]
fn vesuvan_review_blinked_upkeep_target_does_not_supply_its_new_incarnation() {
    let (mut engine, copy) = copy_fixture(llanowar_elves());
    let wall = on_battlefield(&engine, USER, index::WALL_OF_SWORDS).unwrap();
    next_upkeep(&mut engine);
    aim(&mut engine, vec![wall], vec![]);
    priority(&mut engine, USER);
    cast_from_hand(&mut engine, USER, ephemerate());
    aim(&mut engine, vec![wall], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, copy), (1, 1));
    assert_eq!(pt(&engine, wall), (3, 5));
}

fn choose_words(engine: &mut Engine<RegistryLookup>, from: u32, to: u32) {
    assert_ne!(from, to);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 19));
    let choice = from * 4 + if to < from { to } else { to - 1 };
    engine
        .apply(player, PlayerAction::ChooseNumber(choice))
        .unwrap();
}

#[test]
fn hack_review_forest_becomes_island_mana_indefinitely_without_changing_its_name() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), forest()])
        .hand(0, &[index::MAGICAL_HACK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let land = on_battlefield(&engine, USER, forest()).unwrap();
    let name = engine.state().object(land).unwrap().characteristics().name;
    let blue = on_battlefield(&engine, USER, island()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: blue })
        .unwrap();
    cast_with_floating(&mut engine, USER, index::MAGICAL_HACK);
    aim(&mut engine, vec![land], vec![]);
    choose_words(&mut engine, 4, 1);
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, OTHER);
    reach_their_main_phase(&mut engine, USER);
    let c = engine.state().object(land).unwrap().characteristics();
    assert_eq!(c.name, name);
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::land::ISLAND)
    );
    assert!(
        !c.subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST)
    );
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0
    );
}

#[test]
fn hack_review_permanent_spell_carries_landwalk_change_but_blink_does_not() {
    let mut board = vec![swamp(); 4];
    board.extend([island(), island(), plains()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[index::BOG_WRAITH, index::MAGICAL_HACK, ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    for source in all_on_battlefield(&engine, USER, swamp()) {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, USER, index::BOG_WRAITH);
    let wraith = top(&engine);
    let blue = on_battlefield(&engine, USER, island()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: blue })
        .unwrap();
    cast_with_floating(&mut engine, USER, index::MAGICAL_HACK);
    aim(&mut engine, vec![wraith], vec![]);
    choose_words(&mut engine, 2, 4);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wraith), (3, 3));
    assert!(keywords(&engine, wraith).contains(KeywordSet::FORESTWALK));
    assert!(!keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));
    priority(&mut engine, USER);
    let white = on_battlefield(&engine, USER, plains()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: white })
        .unwrap();
    cast_with_floating(&mut engine, USER, ephemerate());
    aim(&mut engine, vec![wraith], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));
    assert!(!keywords(&engine, wraith).contains(KeywordSet::FORESTWALK));
}

#[test]
fn sleight_review_changed_protection_blocks_red_targets_and_allows_black_terror() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), index::WHITE_KNIGHT])
        .hand(0, &[index::SLEIGHT_OF_MIND])
        .battlefield(1, &[mountain(), swamp(), swamp()])
        .hand(1, &[lightning_bolt(), index::TERROR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let knight = on_battlefield(&engine, USER, index::WHITE_KNIGHT).unwrap();
    let name = engine
        .state()
        .object(knight)
        .unwrap()
        .characteristics()
        .name;
    cast_from_hand(&mut engine, USER, index::SLEIGHT_OF_MIND);
    aim(&mut engine, vec![knight], vec![]);
    choose_words(&mut engine, 2, 3);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(knight)
            .unwrap()
            .characteristics()
            .name,
        name
    );
    priority(&mut engine, OTHER);
    // Reserve the black mana for Terror.
    let red = on_battlefield(&engine, OTHER, mountain()).unwrap();
    engine
        .apply(OTHER, PlayerAction::ActivateManaAbility { source: red })
        .unwrap();
    cast_with_floating(&mut engine, OTHER, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Bolt targets expected")
    };
    assert!(!options.contains(&knight));
    aim(&mut engine, vec![], vec![USER]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, index::TERROR);
    aim(&mut engine, vec![knight], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, USER, index::WHITE_KNIGHT).is_some());
}

#[test]
fn sleight_review_changed_spell_keeps_old_targets_and_fails_the_new_color_requirement() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), index::AIR_ELEMENTAL])
        .hand(0, &[index::SLEIGHT_OF_MIND])
        .battlefield(1, &[mountain()])
        .hand(1, &[index::RED_ELEMENTAL_BLAST])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let elemental = on_battlefield(&engine, USER, index::AIR_ELEMENTAL).unwrap();
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, index::RED_ELEMENTAL_BLAST);
    if let Pending::ChooseCastMode { options, .. } = engine.pending() {
        let mode = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
            .unwrap();
        engine.apply(OTHER, PlayerAction::ChooseMode(mode)).unwrap();
    }
    aim(&mut engine, vec![elemental], vec![]);
    let blast = top(&engine);
    priority(&mut engine, USER);
    cast_from_hand(&mut engine, USER, index::SLEIGHT_OF_MIND);
    aim(&mut engine, vec![blast], vec![]);
    choose_words(&mut engine, 1, 4);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elemental), (4, 4));
    assert_eq!(
        engine.state().object(elemental).unwrap().zone,
        Zone::Battlefield
    );
    assert!(in_graveyard(&engine, OTHER, index::RED_ELEMENTAL_BLAST).is_some());
}

fn migration_fixture(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut board = vec![island(); 8];
    board.extend([forest(), llanowar_elves()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

fn enter_copy(engine: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) -> ObjectId {
    priority(engine, USER);
    cast_from_hand(engine, USER, card);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(engine, vec![target], vec![]);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, USER, card).unwrap()
}

#[test]
fn copy_migration_review_clone_inherits_phantasmal_images_targeted_sacrifice() {
    let mut engine = migration_fixture(&[index::PHANTASMAL_IMAGE, index::CLONE]);
    let elf = on_battlefield(&engine, USER, llanowar_elves()).unwrap();
    let image = enter_copy(&mut engine, index::PHANTASMAL_IMAGE, elf);
    let clone = enter_copy(&mut engine, index::CLONE, image);
    assert_eq!(pt(&engine, clone), (1, 1));
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, lightning_bolt());
    aim(&mut engine, vec![clone], vec![]);
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(clone).unwrap().zone == Zone::Graveyard
    });
    assert_eq!(
        engine.state().object(bolt).unwrap().zone,
        Zone::Stack,
        "the copied sacrifice resolves before Bolt can deal lethal damage"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, USER, index::CLONE).is_some());
    assert_eq!(
        engine.state().object(image).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 20);
}

fn creature_count(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            let object = engine.state().object(**id).unwrap();
            object.controller == USER && object.characteristics().types.contains(TypeSet::CREATURE)
        })
        .count()
}

#[test]
fn copy_migration_review_clone_inherits_mimic_token_trigger_but_tokens_do_not_recurse() {
    let mut engine = migration_fixture(&[index::PROGENITOR_MIMIC, index::CLONE]);
    let elf = on_battlefield(&engine, USER, llanowar_elves()).unwrap();
    let mimic = enter_copy(&mut engine, index::PROGENITOR_MIMIC, elf);
    assert_eq!(creature_count(&engine), 2);
    reach_their_main_phase(&mut engine, OTHER);
    reach_their_main_phase(&mut engine, USER);
    assert_eq!(creature_count(&engine), 3);
    let clone = enter_copy(&mut engine, index::CLONE, mimic);
    assert_eq!(pt(&engine, clone), (1, 1));
    assert_eq!(creature_count(&engine), 4);
    for expected in [6, 8] {
        reach_their_main_phase(&mut engine, OTHER);
        reach_their_main_phase(&mut engine, USER);
        assert_eq!(
            creature_count(&engine),
            expected,
            "only the two nontoken copies generate another token each upkeep"
        );
    }
}

#[test]
fn copy_migration_review_copy_artifact_inherits_effigys_added_blue_mana_ability() {
    let mut engine = migration_fixture(&[index::MACHINE_GOD_S_EFFIGY, index::COPY_ARTIFACT]);
    let elf = on_battlefield(&engine, USER, llanowar_elves()).unwrap();
    let effigy = enter_copy(&mut engine, index::MACHINE_GOD_S_EFFIGY, elf);
    priority(&mut engine, USER);
    cast_with_floating(&mut engine, USER, index::COPY_ARTIFACT);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![effigy], vec![]);
    pass_until(&mut engine, stack_is_empty);
    let copy = on_battlefield(&engine, USER, index::COPY_ARTIFACT).unwrap();
    let types = engine.state().object(copy).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::ARTIFACT));
    assert!(types.contains(TypeSet::ENCHANTMENT));
    assert!(!types.contains(TypeSet::CREATURE));
    let blue = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);
    priority(&mut engine, USER);
    engine
        .apply(
            USER,
            PlayerAction::ActivateAbility {
                source: copy,
                ability_index: 1,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        blue + 1
    );
    assert!(is_tapped(&engine, copy));
    assert!(!is_tapped(&engine, effigy));
}
