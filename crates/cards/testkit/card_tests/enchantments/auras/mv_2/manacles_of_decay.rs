//! Oracle scenarios for Manacles of Decay.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn manacles_of_decay() -> CardIndex {
    card_index("69095c25-6ae6-42f5-9c3d-1d8f86f03fae")
}

#[test]
fn manacles_of_decay_prevents_attack_and_grants_cant_block_or_shrinks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[index::PLAINS, index::PLAINS, index::SWAMP, index::MOUNTAIN],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[manacles_of_decay()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut e, p0, manacles_of_decay());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        keywords(&e, giant).contains(KeywordSet::CANT_ATTACK),
        "enchanted creature has can't attack"
    );
    assert_eq!(pt(&e, giant), (3, 3));

    let aura = on_battlefield(&e, p0, manacles_of_decay()).unwrap();
    tap_all_mana(&mut e, p0);

    // {B}: Enchanted creature gets -1/-1 until end of turn (ability_index 2).
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 2,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, giant), (2, 2));

    // {R}: Enchanted creature can't block this turn (ability_index 3).
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 3,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        keywords(&e, giant).contains(KeywordSet::CANT_BLOCK),
        "enchanted creature gained can't block until end of turn"
    );
}

#[test]
fn manacles_of_decay_shrink_can_kill_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[index::PLAINS, index::PLAINS, index::SWAMP, index::SWAMP],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[manacles_of_decay()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut e, p0, manacles_of_decay());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    let aura = on_battlefield(&e, p0, manacles_of_decay()).unwrap();
    tap_all_mana(&mut e, p0);

    for _ in 0..2 {
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: aura,
                ability_index: 2,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }

    assert!(in_graveyard(&e, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, p0, manacles_of_decay()).is_some());
}
