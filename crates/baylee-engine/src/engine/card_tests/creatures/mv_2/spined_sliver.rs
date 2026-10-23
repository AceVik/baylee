//! `cards/creatures/mv_2/spined_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spined_sliver() -> CardIndex {
    card_index("eed9c168-1c78-43c6-a814-9b80054d1373")
}

/// Spined Sliver (`eed9c168-1c78-43c6-a814-9b80054d1373`) — {R}{G} 2/2 Creature — Sliver (Implemented):
/// "Whenever a Sliver becomes blocked, that Sliver gets +1/+1 until end of turn for each creature blocking it."
#[test]
fn spined_sliver_gets_plus_one_plus_one_for_each_creature_blocking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spined_sliver()])
        .battlefield(1, &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sliver = on_battlefield(&engine, P0, spined_sliver()).unwrap();
    let b1 = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    let b2 = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sliver, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, sliver), (b2, sliver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, sliver), (4, 4));
}

#[test]
fn grant_includes_opponents_sliver_but_not_a_nonsliver() {
    for is_sliver in [false, true] {
        let attacker_card = if is_sliver {
            index::METALLIC_SLIVER
        } else {
            index::GRIZZLY_BEARS
        };
        let mut e = lk::game(
            SEED,
            &[attacker_card],
            &[spined_sliver(), index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let attacker = lk::obj(&e, P0, attacker_card);
        let blocker = lk::obj(&e, P1, index::GRIZZLY_BEARS);
        let source = lk::obj(&e, P1, spined_sliver());
        lk::attackers_offered(&mut e);
        lk::declare(&mut e, &[attacker]);
        lk::block(&mut e, &[(blocker, attacker)]);
        lk::settle(&mut e);
        assert_eq!(pt(&e, attacker), (2, 2));
        assert_eq!(
            pt(&e, source),
            (2, 2),
            "grant pumps the attacking Sliver, not its source"
        );
        assert_eq!(pt(&e, blocker), (2, 2));
    }
}
