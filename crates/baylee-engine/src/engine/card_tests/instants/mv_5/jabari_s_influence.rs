//! `cards/instants/mv_5/jabari_s_influence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, CRYSTAL_GOLEM, GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jabari_s_influence() -> CardIndex {
    card_index("8930b1ce-80ac-48b6-a368-ccff9fd01d38")
}

/// Jabari's Influence (8930b1ce-80ac-48b6-a368-ccff9fd01d38) — {3}{W}{W} Instant:
/// "Cast this spell only after combat. Gain control of target nonartifact,
/// nonblack creature that attacked you this turn and put a -1/-0 counter on it."
#[test]
fn jabari_s_influence_gains_control_after_combat_and_adds_minus_one_minus_zero_counter() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[jabari_s_influence()])
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
    cast_with_floating(&mut engine, P0, jabari_s_influence());
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

/// "Cast this spell only after combat" and "nonartifact, nonblack creature that
/// attacked": P1 attacks with Grizzly Bears, a black Bog Imp and an artifact
/// Crystal Golem while a Hill Giant stays home. In the declare blockers step
/// the spell is not castable; after combat only the Bears are on the menu.
#[test]
fn jabari_s_influence_waits_for_combat_and_takes_only_a_nonblack_nonartifact_attacker() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS, BOG_IMP, CRYSTAL_GOLEM, HILL_GIANT])
        .hand(0, &[jabari_s_influence()])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let imp = on_battlefield(&engine, P1, BOG_IMP).unwrap();
    let golem = on_battlefield(&engine, P1, CRYSTAL_GOLEM).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears, Defender::Player(P0)),
                    (imp, Defender::Player(P0)),
                    (golem, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Before combat damage: not castable, with the Plains untapped.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    let card = in_hand(&engine, P0, jabari_s_influence()).unwrap();
    let legal = priority_offer(&engine);
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "only after combat: {legal:?}"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, jabari_s_influence());
    let menu = aim_at(&mut engine, P0, bears);

    assert!(menu.contains(&bears), "{menu:?}");
    assert!(!menu.contains(&imp), "black: {menu:?}");
    assert!(!menu.contains(&golem), "artifact: {menu:?}");
    assert!(!menu.contains(&giant), "did not attack: {menu:?}");
}
