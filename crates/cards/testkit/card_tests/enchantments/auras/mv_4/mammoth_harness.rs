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
        .battlefield(
            1,
            &[bears(), card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let aura = in_hand(&engine, p0, mammoth_harness()).unwrap();
    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let bear = on_battlefield(&engine, p1, bears()).unwrap();
    let vampire = on_battlefield(
        &engine,
        p1,
        card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b"),
    )
    .unwrap();
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
    assert!(
        keywords(&engine, vampire).contains(KeywordSet::FLYING),
        "a flier that is not enchanted keeps flying"
    );

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
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::FIRST_STRIKE),
        "the enchanted creature is not the other creature"
    );
}

/// "Blocks or becomes blocked": the enchanted creature on defence gives the
/// attacker it blocks first strike, and not itself.
#[test]
fn mammoth_harness_grants_first_strike_to_the_attacker_the_enchanted_creature_blocks() {
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

    // P1 attacks with the Bears and the enchanted Angel blocks them.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel, bear)],
            },
        )
        .unwrap();
    // Before combat damage, which kills the Bears.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, bear).contains(KeywordSet::FIRST_STRIKE),
        "the attacker the enchanted Angel blocked gained first strike"
    );
    assert!(!keywords(&engine, angel).contains(KeywordSet::FIRST_STRIKE));
}
