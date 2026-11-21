//! `cards/instants/mv_4/burden_of_greed.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "a80c13db-949d-4784-8177-04aaa22b96b0"
fn burden_of_greed() -> CardIndex {
    card_index("a80c13db-949d-4784-8177-04aaa22b96b0")
}

/// Burden of Greed ({3}{B}): target player loses 1 life for each TAPPED
/// artifact THEY control (ignoring untapped artifacts, tapped lands, and
/// another player's tapped artifacts).
#[test]
fn burden_of_greed_counts_only_targets_tapped_artifacts() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), sol_ring()])
        .hand(0, &[burden_of_greed()])
        .battlefield(1, &[sol_ring(), sol_ring(), sol_ring(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // P1 taps 2 of their 3 Sol Rings plus their Plains (non-artifact).
    let p1_rings = all_on_battlefield(&engine, P1, sol_ring());
    for ring in &p1_rings[..2] {
        engine
            .apply(
                P1,
                PlayerAction::ActivateAbility {
                    source: *ring,
                    ability_index: 0,
                },
            )
            .unwrap();
    }
    let p1_plains = lk::obj(&engine, P1, plains());
    engine
        .apply(P1, PlayerAction::ActivateManaAbility { source: p1_plains })
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // P0 also taps their own Sol Ring (must NOT be counted against P1).
    let p0_ring = lk::obj(&engine, P0, sol_ring());
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: p0_ring,
                ability_index: 0,
            },
        )
        .unwrap();
    let swamps = all_on_battlefield(&engine, P0, swamp());
    tap_mana_where(&mut engine, P0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, P0, burden_of_greed());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lk::life(&engine, P1),
        18,
        "P1 lost exactly 2 life for their 2 tapped Sol Rings"
    );
    assert_eq!(lk::life(&engine, P0), 20);
}
