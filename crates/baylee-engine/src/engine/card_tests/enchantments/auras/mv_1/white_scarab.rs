//! White Scarab — `{W}` Enchantment — Aura (Ice Age)
//!
//! Enchant creature
//! Enchanted creature can't be blocked by white creatures.
//! Enchanted creature gets +2/+2 as long as an opponent controls a white
//! permanent.

#[allow(clippy::wildcard_imports)]
use super::*;

fn white_scarab() -> CardIndex {
    card_index("ec9c3906-8628-48c6-96a7-8c6748cba816")
}

fn tundra_wolves() -> CardIndex {
    card_index("2155864e-5788-4e72-a800-e2cf25cf59a7")
}

#[test]
fn white_scarab_grants_evasion_from_white_creatures_and_conditional_pump_when_opponent_has_white_permanent()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[white_scarab()])
        .hand(1, &[tundra_wolves()])
        .battlefield(0, &[plains(), grizzly_bears(), tundra_wolves()])
        .battlefield(1, &[plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bearer = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let scarab = in_hand(&engine, p0, white_scarab()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: scarab })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bearer],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // p0 controls a white permanent (Tundra Wolves + White Scarab), but p1 does NOT yet control a white permanent.
    assert_eq!(
        pt(&engine, bearer),
        (2, 2),
        "your own white permanents do not turn on White Scarab's +2/+2"
    );

    // Advance to p1's main phase and cast Tundra Wolves (a white permanent).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    let opp_wolves_hand = in_hand(&engine, p1, tundra_wolves()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::CastSpell {
                card: opp_wolves_hand,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let opp_wolves = on_battlefield(&engine, p1, tundra_wolves()).unwrap();

    // Now that opponent p1 controls a white permanent, enchanted Grizzly Bears gets +2/+2 -> 4/4!
    assert_eq!(
        pt(&engine, bearer),
        (4, 4),
        "enchanted creature gets +2/+2 while an opponent controls a white permanent"
    );

    // Advance to p0's next combat and attack with bearer: white Tundra Wolves cannot block, green Grizzly Bears can.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    let blocks = attack_and_collect_blocks(&mut engine, bearer, p1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == opp_wolves && b.attackers.contains(&bearer)),
        "white Tundra Wolves cannot block creature enchanted by White Scarab"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == opp_bears && b.attackers.contains(&bearer)),
        "non-white Grizzly Bears can still block creature enchanted by White Scarab"
    );
}
