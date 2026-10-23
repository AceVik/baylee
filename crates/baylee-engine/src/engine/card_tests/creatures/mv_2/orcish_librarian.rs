//! `cards/creatures/mv_2/orcish_librarian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn orcish_librarian() -> CardIndex {
    card_index("3a05904d-5439-446d-a2a1-1475c9c7371d")
}

/// Orcish Librarian (3a05904d-5439-446d-a2a1-1475c9c7371d) — {1}{R} 1/1 Creature — Orc:
/// `Coverage::Partial` — casts for {1}{R} as a 1/1 Orc.
#[test]
fn orcish_librarian_casts_for_one_and_a_red_as_a_one_one_orc() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[orcish_librarian()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, orcish_librarian());
    pass_until(&mut engine, stack_is_empty);

    let orc = on_battlefield(&engine, P0, orcish_librarian()).unwrap();
    assert_eq!(pt(&engine, orc), (1, 1));
}
