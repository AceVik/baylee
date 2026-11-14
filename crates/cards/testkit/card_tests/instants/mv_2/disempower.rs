//! `cards/1996/src/mir/disempower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn disempower() -> CardIndex {
    card_index("ff9f4bfe-1585-4eaa-943c-1e85ed79a820")
}

/// Disempower puts target artifact or enchantment on top of its owner's
/// library and rejects non-artifact, non-enchantment permanents.
#[test]
fn disempower_puts_artifact_or_enchantment_on_top_of_owners_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(
            1,
            &[
                quiet_artifact(),
                their_enchantment(),
                grizzly_bears(),
                forest(),
            ],
        )
        .hand(0, &[disempower(), disempower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ring = on_battlefield(&engine, P1, quiet_artifact()).unwrap();
    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let their_forest = on_battlefield(&engine, P1, forest()).unwrap();

    // First cast targets the artifact: excludes creature & land, offers both artifact and enchantment.
    cast_from_hand(&mut engine, P0, disempower());
    let options = aim_at(&mut engine, P0, ring);
    assert!(options.contains(&ring) && options.contains(&chant));
    assert!(!options.contains(&bears) && !options.contains(&their_forest));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, quiet_artifact()).is_none());
    assert!(in_graveyard(&engine, P1, quiet_artifact()).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(ring)
    );

    // Second cast targets the enchantment -> sits directly on top above the artifact.
    cast_from_hand(&mut engine, P0, disempower());
    aim_at(&mut engine, P0, chant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, their_enchantment()).is_none());
    let lib = engine.state().zones.list(ZoneLocation::Library(P1));
    assert_eq!(lib.last().copied(), Some(chant));
    assert_eq!(lib.get(lib.len() - 2).copied(), Some(ring));

    // On P1's draw step, P1 draws that exact enchantment back to hand.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, their_enchantment()), Some(chant));
}
