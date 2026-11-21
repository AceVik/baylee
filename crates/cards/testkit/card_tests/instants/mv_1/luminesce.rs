//! `cards/instants/mv_1/luminesce.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "7b0f482d-35a3-47f1-8283-0fd47e30cc31"
fn luminesce() -> CardIndex {
    card_index("7b0f482d-35a3-47f1-8283-0fd47e30cc31")
}

/// Luminesce ({W}): prevents all combat AND noncombat damage from black sources
/// and red sources this turn, does NOT prevent green damage, and expires at EOT.
#[test]
fn luminesce_prevents_black_and_red_damage_only_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            index::SCATHE_ZOMBIES,
            index::HILL_GIANT,
            index::GRIZZLY_BEARS,
        ],
        &[luminesce()],
        &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
    );
    let zombies = lk::obj(&engine, P1, index::SCATHE_ZOMBIES);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    reach_their_main_phase(&mut engine, P1);

    // P1 passes priority to P0; P0 casts Luminesce.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut engine, P0, luminesce());
    lk::settle(&mut engine);

    // Noncombat red spell damage is prevented.
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "red spell damage prevented");

    // Combat: black 2/2 and red 3/3 are prevented; green 2/2 deals 2 damage.
    let _ = lk::attack(&mut engine, &[zombies, giant, bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "only green Bears dealt 2; black Zombies and red Giant prevented"
    );

    // Next turn, Luminesce has expired: red Lightning Bolt deals full 3 damage.
    reach_their_main_phase(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 15, "prevention wore off next turn");
}
