//! `cards/2001/src/ody/repel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn repel() -> CardIndex {
    card_index("6e072b3a-7a83-4d77-bae3-72de0e646b81")
}

/// Repel targets only creatures (never artifacts, enchantments, or lands), puts
/// the chosen creature on top of its owner's library, and leaves it as the
/// next card drawn.
#[test]
fn repel_puts_target_creature_on_top_of_owners_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 4])
        .battlefield(
            1,
            &[
                grizzly_bears(),
                quiet_artifact(),
                their_enchantment(),
                forest(),
            ],
        )
        .hand(0, &[repel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let ring = on_battlefield(&engine, P1, quiet_artifact()).unwrap();
    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let land = on_battlefield(&engine, P1, forest()).unwrap();

    cast_from_hand(&mut engine, P0, repel());
    let options = aim_at(&mut engine, P0, bears);
    assert_eq!(options, vec![bears], "only creatures may be targeted");
    assert!(!options.contains(&ring) && !options.contains(&chant) && !options.contains(&land));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(bears)
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, grizzly_bears()), Some(bears));
}
