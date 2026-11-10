//! `cards/creatures/mv_4/deathmark_prelate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Deathmark Prelate (`3162e0b4-1348-4c86-8da7-ae23ff968c57`) — {3}{B} Creature — Human Cleric 2/3:
/// "{2}{B}, {T}, Sacrifice a Zombie: Destroy target non-Zombie creature. It can't be regenerated. Activate only as a sorcery."
#[test]
fn deathmark_prelate_sacrifices_zombie_to_destroy_non_zombie_without_regeneration() {
    let prelate_card = card_index("3162e0b4-1348-4c86-8da7-ae23ff968c57");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                prelate_card,
                index::SCATHE_ZOMBIES,
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[index::SCATHE_ZOMBIES, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, index::SCATHE_ZOMBIES).unwrap();
    let opp_zombie = on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(opp_bear)
        .unwrap()
        .regeneration_shields = 1;

    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, prelate_card, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Deathmark Prelate");
    };
    assert!(options.contains(&opp_bear));
    assert!(!options.contains(&opp_zombie));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bear],
            },
        )
        .unwrap();

    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, index::SCATHE_ZOMBIES).is_some());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, index::SCATHE_ZOMBIES).is_some());
}
