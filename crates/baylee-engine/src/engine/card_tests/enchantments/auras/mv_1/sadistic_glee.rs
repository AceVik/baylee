//! `cards/enchantments/auras/mv_1/sadistic_glee.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT, LLANOWAR_ELVES};

fn sadistic_glee() -> CardIndex {
    card_index("2af22962-8f66-4868-a175-ad5d7f4c9d58")
}

/// `Sadistic Glee` ({B}, Enchantment — Aura, `Coverage::Implemented`):
/// `Enchant creature`
/// `Whenever a creature dies, put a +1/+1 counter on enchanted creature.`
#[test]
fn sadistic_glee_puts_a_plus_one_plus_one_counter_on_enchanted_creature_when_a_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), GRIZZLY_BEARS])
        .battlefield(1, &[LLANOWAR_ELVES])
        .hand(0, &[sadistic_glee(), LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, p1, LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, p0, sadistic_glee());
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
    let ch = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));

    // Kill Llanowar Elves -> Sadistic Glee triggers and puts a +1/+1 counter on Grizzly Bears.
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ch = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
}

#[test]
fn your_creature_dying_also_grows_an_enchanted_opposing_creature() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[LLANOWAR_ELVES],
        &[GRIZZLY_BEARS],
        &[sadistic_glee(), LIGHTNING_BOLT],
        &[],
    );
    let bears = on_battlefield(&e, p1, GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&e, p0, LLANOWAR_ELVES).unwrap();
    lk::float(&mut e, p0, &[(ManaColor::Black, 1), (ManaColor::Red, 1)]);
    cast_with_floating(&mut e, p0, sadistic_glee());
    lk::aim(&mut e, &[bears], &[]);
    lk::settle(&mut e);
    cast_with_floating(&mut e, p0, LIGHTNING_BOLT);
    lk::aim(&mut e, &[elves], &[]);
    lk::settle(&mut e);
    let ch = e.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
}
