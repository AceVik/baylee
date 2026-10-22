//! `cards/instants/mv_5/jabari_s_influence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JABARI_S_INFLUENCE};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Jabari's Influence (c9f76dd4-5f1e-44fb-8c98-958a721a15f8) — {3}{W}{W} Instant:
/// "Cast this spell only after combat. Gain control of target nonartifact,
/// nonblack creature that attacked you this turn and put a -1/-0 counter on it."
#[test]
fn jabari_s_influence_gains_control_after_combat_and_adds_minus_one_minus_zero_counter() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[JABARI_S_INFLUENCE])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    // Advance to P1's DeclareAttackers step and attack P0 with Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, JABARI_S_INFLUENCE);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    let obj = engine.state().object(bears).unwrap();
    assert_eq!(obj.controller, P0);
    assert_eq!(
        obj.counters.get(CounterKind::Minus {
            power: 1,
            toughness: 0,
        }),
        1
    );
    let chars = obj.characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(2)));
}
