//! `cards/creatures/mv_1/accursed_centaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn accursed_centaur_sacrifices_a_creature_when_it_enters() {
    let p0 = PlayerId::new(0);
    let centaur = card_index("b1e6921e-e460-4e58-82b0-d98ee31c279d");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[centaur])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, centaur);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, centaur).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}
