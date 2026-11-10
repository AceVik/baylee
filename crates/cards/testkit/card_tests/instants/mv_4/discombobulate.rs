//! `cards/instants/mv_4/discombobulate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn discombobulate_counters_target_spell_and_reorders_top_four_of_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let discombobulate = card_index("b58d7a20-4bcd-4c33-8cda-955362525f48");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[forest()])
        .hand(0, &[discombobulate])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, llanowar_elves());
    let spell_id = on_stack(&engine, llanowar_elves()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let lib_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, discombobulate);
    aim_at(&mut engine, p0, spell_id);
    assert!(matches!(drive_to_rest(&mut engine, p1), Rest::Reached));

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(library_size(&engine, p0), lib_before);
}
