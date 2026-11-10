//! `cards/sorceries/mv_4/flameshot.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn flameshot() -> CardIndex {
    card_index("26640648-6400-4b80-a78b-f519ab9cd09e")
}

/// Flameshot ({3}{R} Sorcery, Partial):
/// "Flameshot deals 3 damage divided as you choose among one, two, or three target creatures."
#[test]
fn flameshot_deals_3_damage_to_single_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[flameshot()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, flameshot());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

/// Flameshot can divide its 3 damage across two target creatures (2 and 1).
#[test]
fn flameshot_divides_damage_across_two_target_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[flameshot()])
        .battlefield(1, &[index::GRIZZLY_BEARS, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, flameshot());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear, elf],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
}
