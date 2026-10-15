//! The Abyss: upkeep targeting, chooser identity, and unregenerable destruction.
// oracle_id = "a02a7816-c967-4503-bb08-f8db44915250"

#[allow(clippy::wildcard_imports)] // Shared canonical-card vocabulary.
use super::*;
use crate::engine::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn target_menu(e: &mut Engine<RegistryLookup>, chooser: PlayerId) -> Vec<ObjectId> {
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, chooser);
    assert_eq!((*min, *max), (1, 1));
    options.clone()
}

fn aim(e: &mut Engine<RegistryLookup>, chooser: PlayerId, target: ObjectId) {
    e.apply(
        chooser,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
}

#[test]
fn batch60_abyss_each_upkeep_active_player_targets_only_their_nonartifact_creature() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::THE_ABYSS,
                index::GRIZZLY_BEARS,
                index::CRAW_WURM,
                index::ORNITHOPTER,
            ],
        )
        .battlefield(
            1,
            &[
                index::GRIZZLY_BEARS,
                index::CRAW_WURM,
                index::ORNITHOPTER,
                index::FOREST,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    let own = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&e, P1, index::CRAW_WURM).unwrap();
    let first = target_menu(&mut e, P0);
    assert_eq!(first.len(), 2);
    assert!(first.contains(&own));
    assert!(!first.contains(&other));
    aim(&mut e, P0, own);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
    let second = target_menu(&mut e, P1);
    assert_eq!(second.len(), 2);
    assert!(second.contains(&other));
    aim(&mut e, P1, other);
    let trigger = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    assert_eq!(
        e.state().object(trigger).unwrap().controller,
        P0,
        "active player chooses, but Abyss owner controls the ability"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, index::CRAW_WURM).is_some());
    assert!(on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::ORNITHOPTER).is_some());
}

#[test]
fn batch60_abyss_chooser_does_not_change_hexproof_shroud_or_protection_legality() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::THE_ABYSS,
                index::SLIPPERY_BOGLE,
                index::DEADLY_INSECT,
                index::WHITE_KNIGHT,
            ],
        )
        .battlefield(
            1,
            &[
                index::GRIZZLY_BEARS,
                index::SLIPPERY_BOGLE,
                index::DEADLY_INSECT,
                index::WHITE_KNIGHT,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    let bogle = on_battlefield(&e, P0, index::SLIPPERY_BOGLE).unwrap();
    assert_eq!(target_menu(&mut e, P0), vec![bogle]);
    aim(&mut e, P0, bogle);
    pass_until(&mut e, stack_is_empty);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(target_menu(&mut e, P1), vec![bear]);
    aim(&mut e, P1, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::SLIPPERY_BOGLE).is_some());
}

#[test]
fn batch60_abyss_regeneration_cannot_save_target_and_source_departure_does_not_cancel_trigger() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[index::THE_ABYSS, index::DRUDGE_SKELETONS, index::SWAMP],
        )
        .battlefield(1, &[index::PLAINS])
        .hand(1, &[index::DISENCHANT])
        .start();
    keep_mulligans(&mut e);
    let skeleton = on_battlefield(&e, P0, index::DRUDGE_SKELETONS).unwrap();
    let abyss = on_battlefield(&e, P0, index::THE_ABYSS).unwrap();
    assert_eq!(target_menu(&mut e, P0), vec![skeleton]);
    aim(&mut e, P0, skeleton);
    lk::float(&mut e, P0, &[(ManaColor::Black, 1)]);
    kit::activate_object(&mut e, P0, skeleton, 0);
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
            && matches!(e.pending(), Pending::Priority { .. })
    });
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut e, P1, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut e, P1, index::DISENCHANT);
    aim(&mut e, P1, abyss);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::THE_ABYSS).is_some());
    assert!(in_graveyard(&e, P0, index::DRUDGE_SKELETONS).is_some());
}

#[test]
fn batch60_abyss_target_that_becomes_artifact_is_illegal_on_resolution() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::THE_ABYSS,
                index::GRIZZLY_BEARS,
                index::ASHNOD_S_TRANSMOGRANT,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let transmogrant = on_battlefield(&e, P0, index::ASHNOD_S_TRANSMOGRANT).unwrap();
    assert_eq!(target_menu(&mut e, P0), vec![bear]);
    aim(&mut e, P0, bear);
    kit::activate_object(&mut e, P0, transmogrant, 0);
    aim(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(
        e.state()
            .object(bear)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );
    assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 1);
}

#[test]
fn batch60_abyss_no_legal_creatures_does_not_request_target_and_indestructible_survives() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[index::THE_ABYSS, index::DEADLY_INSECT, index::ORNITHOPTER],
        )
        .battlefield(1, &[index::AVACYN_ANGEL_OF_HOPE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let avacyn = on_battlefield(&e, P1, index::AVACYN_ANGEL_OF_HOPE).unwrap();
    assert_eq!(target_menu(&mut e, P1), vec![avacyn]);
    aim(&mut e, P1, avacyn);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::DEADLY_INSECT).is_some());
    assert!(on_battlefield(&e, P1, index::AVACYN_ANGEL_OF_HOPE).is_some());
}

#[test]
fn batch60_abyss_control_change_rechecks_original_upkeep_player() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[index::OLD_MAN_OF_THE_SEA, index::GRIZZLY_BEARS])
        .hand(0, &[index::THE_ABYSS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    lk::float(&mut e, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e, P0, index::THE_ABYSS);
    pass_until(&mut e, stack_is_empty);
    let spare = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    assert!(target_menu(&mut e, P1).contains(&spare));
    aim(&mut e, P1, spare);
    pass_until(&mut e, stack_is_empty);
    // Its controller has now begun a turn, so Old Man can pay its tap cost.
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let old_man = on_battlefield(&e, P1, index::OLD_MAN_OF_THE_SEA).unwrap();
    assert_eq!(target_menu(&mut e, P0), vec![bear]);
    aim(&mut e, P0, bear);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    kit::activate_object(&mut e, P1, old_man, 1);
    aim(&mut e, P1, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, old_man));
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(bear).unwrap().controller, P1);
}

#[test]
fn batch60_abyss_world_rule_keeps_only_newest_world_permanent() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::THE_ABYSS])
        .hand(0, &[index::NETHER_VOID])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    lk::float(&mut e, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut e, P0, index::NETHER_VOID);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::THE_ABYSS).is_some());
    assert!(on_battlefield(&e, P0, index::NETHER_VOID).is_some());
}
