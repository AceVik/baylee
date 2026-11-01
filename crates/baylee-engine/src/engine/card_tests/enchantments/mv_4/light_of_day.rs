//! `cards/enchantments/mv_4/light_of_day.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit::may_block;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn light_of_day() -> CardIndex {
    card_index("cd854e00-c9da-4593-8402-4bae637f5185")
}

/// Light of Day (`cd854e00-c9da-4593-8402-4bae637f5185`) — {3}{W} Enchantment:
/// "Black creatures can't attack or block."
#[test]
fn light_of_day_prevents_black_creatures_from_attacking_or_blocking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                index::GRIZZLY_BEARS,
                index::SCATHE_ZOMBIES,
            ],
        )
        .battlefield(1, &[index::SCATHE_ZOMBIES, index::HILL_GIANT])
        .hand(0, &[light_of_day()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let p0_zombies = on_battlefield(&engine, P0, index::SCATHE_ZOMBIES).unwrap();
    let p1_zombies = on_battlefield(&engine, P1, index::SCATHE_ZOMBIES).unwrap();
    let p1_giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, light_of_day());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(attackers.contains(&p0_bears));
    assert!(!attackers.contains(&p0_zombies));

    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p0_bears, Defender::Player(P1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers");
    };
    assert!(may_block(&blockers, p1_giant, p0_bears));
    assert!(!may_block(&blockers, p1_zombies, p0_bears));
}
