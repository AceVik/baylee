//! `cards/instants/mv_3/infuse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infuse() -> CardIndex {
    card_index("6035303e-22b3-4b64-8160-1f46ae3dad55")
}

/// Infuse (6035303e-22b3-4b64-8160-1f46ae3dad55) — {2}{U} Instant:
/// - "Untap target artifact, creature, or land."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn infuse_untaps_artifact_creature_or_land_excludes_enchantment_and_draws_on_next_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                sol_ring(),
                llanowar_elves(),
                index::CRUSADE,
            ],
        )
        .hand(0, &[infuse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ring = on_battlefield(&engine, P0, sol_ring()).expect("Sol Ring seated");
    let elf = on_battlefield(&engine, P0, llanowar_elves()).expect("Elf seated");
    let crusade = on_battlefield(&engine, P0, index::CRUSADE).expect("Crusade seated");

    // Tap Sol Ring and the 3 Islands.
    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    assert!(is_tapped(&engine, ring));

    cast_from_hand(&mut engine, P0, infuse());
    let options = aim_at(&mut engine, P0, ring);
    assert!(
        options.contains(&ring) && options.contains(&elf),
        "artifacts, creatures, and lands are legal targets"
    );
    assert!(
        !options.contains(&crusade),
        "a non-artifact, non-creature enchantment is not a legal target"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, ring),
        "Infuse untaps the targeted Sol Ring"
    );
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    // At the beginning of the next turn's (P1's) upkeep, P0 draws 1 card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_after_cast + 1,
        "delayed trigger draws a card at the beginning of the next turn's upkeep"
    );
}
