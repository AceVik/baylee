//! `cards/sorceries/mv_2/chain_of_smog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chain_of_smog() -> CardIndex {
    card_index("ea14c26b-bf2f-48b4-b879-6e63069ded1f")
}

fn resolve_discard_two(engine: &mut Engine<RegistryLookup>, who: PlayerId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, who);
    assert_eq!((min, max), (2, 2));
    engine
        .apply(
            who,
            PlayerAction::ChooseObjects {
                objects: vec![options[0], options[1]],
            },
        )
        .unwrap();
}

/// Chain of Smog (ea14c26b-bf2f-48b4-b879-6e63069ded1f) — {1}{B} Sorcery:
/// "Target player discards two cards. That player may copy this spell and may
/// choose a new target for that copy."
#[test]
fn chain_of_smog_makes_target_discard_two_and_lets_them_copy_back_at_caster() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp()],
        &[],
        &[chain_of_smog(), forest(), island(), plains()],
        &[forest(), island(), mountain()],
    );

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, chain_of_smog(), 0, 0, &[], &[P1]);
    resolve_discard_two(&mut engine, P1);

    // P1 accepts copying Chain of Smog and retargets the copy at P0.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player: copier, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(copier, P1);
    lk::yes_no(&mut engine, true);

    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(&mut engine, true),
            Pending::ChoosePlayer { player, .. } => {
                assert_eq!(player, P1);
                engine.apply(P1, PlayerAction::ChoosePlayer(P0)).unwrap();
                break;
            }
            Pending::ChooseTargets { player, .. } => {
                assert_eq!(player, P1);
                lk::aim(&mut engine, &[], &[P0]);
                break;
            }
            _ => break,
        }
    }

    // Copy resolves: P0 discards 2 cards, then declines copying further.
    resolve_discard_two(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
}
