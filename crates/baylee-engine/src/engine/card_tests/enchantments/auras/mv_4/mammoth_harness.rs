//! Mammoth Harness — `{3}{G}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! Enchanted creature loses flying.
//! Whenever enchanted creature blocks or becomes blocked by a creature, the
//! other creature gains first strike until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn mammoth_harness() -> CardIndex {
    card_index("f2e106c6-7666-42cd-9a02-e51327fb80e8")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn mammoth_harness_removes_flying_and_grants_blocker_first_strike() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[mammoth_harness()])
        .battlefield(0, &[serra_angel(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let aura = in_hand(&engine, p0, mammoth_harness()).unwrap();
    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let bear = on_battlefield(&engine, p1, bears()).unwrap();
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: aura })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));

    // Attack with Serra Angel; Grizzly Bears (non-flyer) can now block it and gains First Strike.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, angel)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bear).contains(KeywordSet::FIRST_STRIKE));
}
