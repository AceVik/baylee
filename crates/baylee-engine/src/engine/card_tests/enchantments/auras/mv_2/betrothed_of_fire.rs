//! `cards/enchantments/auras/mv_2/betrothed_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MONS_S_GOBLIN_RAIDERS};

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
