//! `cards/instants/mv_1/fighting_chance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fighting_chance() -> CardIndex {
    card_index("fece632b-d8d5-4d3d-a4a7-643293b67538")
}

/// Fighting Chance (`fece632b-d8d5-4d3d-a4a7-643293b67538`) — {R} Instant (Implemented):
/// "For each blocking creature, flip a coin. If you win the flip, prevent all combat damage that would be dealt by that creature this turn."
#[test]
fn fighting_chance_flips_a_coin_for_each_blocking_creature_and_prevents_combat_damage_on_win() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[index::GRIZZLY_BEARS, mountain()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[fighting_chance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let attacker = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    kit::declare_attack(&mut engine, &[attacker], P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    cast_from_hand(&mut engine, P0, fighting_chance());
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert!(in_graveyard(&engine, P0, fighting_chance()).is_some());
}
