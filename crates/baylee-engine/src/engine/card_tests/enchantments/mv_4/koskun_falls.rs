//! Koskun Falls — `{2}{B}{B}` World Enchantment (Homelands)
//!
//! At the beginning of your upkeep, sacrifice this enchantment unless you tap
//! an untapped creature you control.
//! Creatures can't attack you unless their controller pays `{2}` for each
//! creature they control that's attacking you.
//!
//! (`Coverage::Partial`: `CantAttackUnlessPayMana` taxes all attacks by
//! opposing creatures rather than only creatures attacking you directly;
//! upkeep tap-a-creature-or-sacrifice and `{2}` attack tax on opponents'
//! creatures work.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn koskun_falls() -> CardIndex {
    card_index("a9b9e798-debc-4f2c-9770-e16f670082f7")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn koskun_falls_requires_opponent_to_pay_two_per_attacker_and_sacrifices_on_upkeep_without_creature()
 {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[koskun_falls()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[bears(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Koskun Falls in P0's FirstMain so it enters after P0's Turn 1 upkeep.
    let falls_card = in_hand(&engine, p0, koskun_falls()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: falls_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, koskun_falls()).is_some());

    // Pass to P1's DeclareAttackers step; P1 only has 1 Forest so cannot pay {2} to attack.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bear = on_battlefield(&engine, p1, bears()).unwrap();
    let res = engine.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0))],
        },
    );
    assert!(
        res.is_err(),
        "Grizzly Bears cannot pay {{2}} for Koskun Falls with only 1 Forest"
    );

    // Advance to P0's next FirstMain with no creatures to tap on upkeep: PlayerMayPayCostOr skips the unpayable cost and sacrifices Koskun Falls!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(on_battlefield(&engine, p0, koskun_falls()).is_none());
    assert!(in_graveyard(&engine, p0, koskun_falls()).is_some());
}
