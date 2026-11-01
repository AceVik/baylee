//! `cards/enchantments/auras/mv_2/betrothed_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, MONS_S_GOBLIN_RAIDERS};

fn betrothed_of_fire() -> CardIndex {
    card_index("77849657-c872-4d54-9149-a08bbe90f787")
}

/// `Betrothed of Fire` ({1}{R}, Enchantment — Aura) sacrifices an untapped
/// creature to give enchanted creature +2/+0 until end of turn.
#[test]
fn betrothed_of_fire_sacrifices_untapped_creature_to_pump_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), GRIZZLY_BEARS, MONS_S_GOBLIN_RAIDERS],
        )
        .hand(0, &[betrothed_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let raiders = on_battlefield(&engine, p0, MONS_S_GOBLIN_RAIDERS).unwrap();

    cast_from_hand(&mut engine, p0, betrothed_of_fire());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, betrothed_of_fire(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raiders],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, MONS_S_GOBLIN_RAIDERS).is_some());
    assert_eq!(pt(&engine, bears), (4, 2));
}

/// Casts Betrothed of Fire from `p0`'s hand on `target` and lets it resolve.
#[track_caller]
fn betroth(engine: &mut Engine<RegistryLookup>, p0: PlayerId, target: ObjectId) {
    cast_from_hand(engine, p0, betrothed_of_fire());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// "Sacrifice an **untapped** creature": the menu holds the caster's untapped
/// creatures only. The tapped Goblin and the opponent's Giant are not on it,
/// the Giant sacrificed is the caster's own, and only the *enchanted* Bears
/// grows.
#[test]
fn betrothed_of_fire_first_cost_offers_only_untapped_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                GRIZZLY_BEARS,
                MONS_S_GOBLIN_RAIDERS,
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[betrothed_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let raiders = on_battlefield(&engine, p0, MONS_S_GOBLIN_RAIDERS).unwrap();
    let mine = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let theirs = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    betroth(&mut engine, p0, bears);
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(raiders)
        .expect("the Goblin")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    activate(&mut engine, p0, betrothed_of_fire(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&bears) && options.contains(&mine));
    assert!(
        !options.contains(&raiders),
        "a tapped creature is not offered"
    );
    assert!(!options.contains(&theirs), "nor is the opponent's creature");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_some());
    assert_eq!(
        pt(&engine, bears),
        (4, 2),
        "the enchanted creature gets +2/+0"
    );
    assert_eq!(pt(&engine, raiders), (1, 1), "and nothing else does");
}

/// "Sacrifice enchanted creature: Creatures you control get +2/+0 until end of
/// turn": the Bears go, the caster's other creatures grow, the opponent's do
/// not, and the bonus ends with the turn.
#[test]
fn betrothed_of_fire_second_ability_pumps_the_rest_of_your_creatures_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), GRIZZLY_BEARS, MONS_S_GOBLIN_RAIDERS],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[betrothed_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let raiders = on_battlefield(&engine, p0, MONS_S_GOBLIN_RAIDERS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    betroth(&mut engine, p0, bears);

    activate(&mut engine, p0, betrothed_of_fire(), 2);
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert_eq!(
            options,
            vec![bears],
            "only the enchanted creature is the cost"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert_eq!(
        pt(&engine, raiders),
        (3, 1),
        "your other creatures get +2/+0"
    );
    assert_eq!(pt(&engine, giant), (3, 3), "the opponent's do not");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, raiders), (1, 1), "until end of turn");
}

/// On an opponent's creature the Aura still pumps it (the first ability), but
/// "sacrifice enchanted creature" is a cost only the creature's controller
/// can pay, so the second ability is not offered.
#[test]
fn betrothed_of_fire_on_an_opponents_creature_cannot_sacrifice_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), GRIZZLY_BEARS, MONS_S_GOBLIN_RAIDERS],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[betrothed_of_fire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    betroth(&mut engine, p0, giant);

    assert!(lk::offered(&engine, betrothed_of_fire(), p0, 1));
    assert!(
        !lk::offered(&engine, betrothed_of_fire(), p0, 2),
        "the Giant is not ours to sacrifice"
    );
    activate(&mut engine, p0, betrothed_of_fire(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice choice, got {:?}", engine.pending())
    };
    assert!(!options.contains(&giant));
    let raiders = on_battlefield(&engine, p0, MONS_S_GOBLIN_RAIDERS).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raiders],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (5, 3));
}
