//! `cards/creatures/mv_7/havoc_demon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Havoc Demon (`246d7f6b-7ffa-46dc-b5cb-6959f47a7dc3`) — {5}{B}{B} Creature — Demon 5/5:
/// "Flying
/// When this creature dies, all creatures get -5/-5 until end of turn."
#[test]
fn havoc_demon_gives_all_creatures_minus_five_minus_five_when_it_dies() {
    let demon_card = card_index("246d7f6b-7ffa-46dc-b5cb-6959f47a7dc3");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[demon_card])
        .battlefield(1, &[index::HILL_GIANT, index::FORCE_OF_NATURE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let demon = on_battlefield(&engine, p0, demon_card).unwrap();
    let force = on_battlefield(&engine, p1, index::FORCE_OF_NATURE).unwrap();
    assert!(keywords(&engine, demon).contains(KeywordSet::FLYING));

    kill(&mut engine, demon);

    assert!(in_graveyard(&engine, p0, demon_card).is_some());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p1, index::FORCE_OF_NATURE).is_some());
    assert_eq!(pt(&engine, force), (3, 3));
}
