//! `cards/creatures/mv_5/krovikan_vampire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krovikan_vampire() -> CardIndex {
    card_index("8fa93c01-f9ea-4d48-a040-418d1ca6be19")
}

/// Krovikan Vampire (8fa93c01-f9ea-4d48-a040-418d1ca6be19) — {3}{B}{B} 3/3 Creature — Vampire:
/// - "At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature."
///   (Partial: plays as a 3/3 Vampire with no abilities.)
#[test]
fn krovikan_vampire_casts_for_three_and_two_black_and_fights_as_a_three_three_vampire() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
            ],
        )
        .hand(0, &[krovikan_vampire()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, krovikan_vampire());
    pass_until(&mut engine, stack_is_empty);

    let vampire = on_battlefield(&engine, P0, krovikan_vampire()).expect("Vampire entered");
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).expect("Bears seated");
    assert_eq!(pt(&engine, vampire), (3, 3));

    // Advance to P0's next turn so Krovikan Vampire loses summoning sickness and attacks.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let _ = lk::attack(&mut engine, &[vampire]);
    lk::block(&mut engine, &[(bears, vampire)]);
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "3/3 Krovikan Vampire kills the blocking 2/2 Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P0, krovikan_vampire()).is_some(),
        "3/3 Krovikan Vampire survives 2 combat damage"
    );
}
