//! `cards/sorceries/mv_1/reckless_abandon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reckless_abandon() -> CardIndex {
    card_index("f24d5187-0f80-4528-bf44-03ae146f3551")
}

/// Reckless Abandon (f24d5187-0f80-4528-bf44-03ae146f3551) — {R} Sorcery:
/// "As an additional cost to cast this spell, sacrifice a creature."
/// "Reckless Abandon deals 4 damage to any target."
#[test]
fn reckless_abandon_requires_sacrificing_your_creature_and_deals_4_damage_to_any_target() {
    use baylee_core::generated::index;

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[reckless_abandon(), reckless_abandon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let their_giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    // Cast first Reckless Abandon targeting the opponent's 3/3 Hill Giant,
    // sacrificing our own Grizzly Bears (opponent's creature is not offered for sacrifice).
    tap_all_mana(&mut engine, P0);
    lk::announce(
        &mut engine,
        P0,
        reckless_abandon(),
        0,
        0,
        &[their_giant],
        &[],
    );

    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert!(
        options.contains(&my_bears),
        "own creature is in sacrifice options"
    );
    assert!(
        !options.contains(&their_giant),
        "opponent's creature cannot be sacrificed to pay the additional cost"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![my_bears],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(my_bears).unwrap().zone,
        Zone::Graveyard,
        "sacrificed creature moves to the graveyard on announcement"
    );
    lk::settle(&mut engine);
    assert_eq!(
        engine.state().object(their_giant).unwrap().zone,
        Zone::Graveyard,
        "4 damage destroys the 3/3 Hill Giant"
    );

    // Negative boundary: with no creature left under P0's control, the second
    // Reckless Abandon cannot be cast even with floating red mana.
    assert!(
        !lk::castable(&engine, P0, reckless_abandon()),
        "cannot cast Reckless Abandon without a creature you control to sacrifice"
    );
}
