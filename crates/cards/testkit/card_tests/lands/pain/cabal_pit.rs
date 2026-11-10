//! `cards/lands/pain/cabal_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cabal Pit prints `{{T}}: Add {{B}}. This land deals 1 damage to you.` and
/// `Threshold — {{B}}, {{T}}, Sacrifice this land: Target creature gets -2/-2
/// until end of turn. Activate only if there are seven or more cards in your graveyard.`
///
/// Under `Coverage::Partial`, the threshold gate cannot be checked by the engine
/// and the sacrifice ability is offered unconditionally. This test floats `{B}`
/// from a Swamp, activates ability 1 targeting an `aurochs()`, confirms that
/// Cabal Pit is sacrificed as a cost before resolution, and verifies that the
/// target creature receives -2/-2.
#[test]
fn cabal_pit_activates_to_give_target_creature_minus_two_minus_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cabal_pit(), swamp(), aurochs()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven cards, because the ability is gated on threshold now.
    // This test read the ability as offered on an empty graveyard for
    // as long as `Condition` could not say the sentence, which is a
    // land strictly stronger than the printed one.
    seed_graveyard(&mut engine, p0, 7);

    let cow = on_battlefield(&engine, p0, aurochs()).expect("aurochs on battlefield");
    assert_eq!(pt(&engine, cow), (2, 3));

    tap_all_mana_but(&mut engine, p0, Some(cabal_pit()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );

    activate(&mut engine, p0, cabal_pit(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cow));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![cow] })
        .unwrap();

    assert!(
        in_graveyard(&engine, p0, cabal_pit()).is_some(),
        "cabal pit sacrificed as cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana spent to pay activation cost"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, cow), (0, 1), "aurochs received -2/-2");
}

// ── Threshold: the gate that was missing, and the six lands behind it ─────

/// Cabal Pit at the boundary — six cards is not threshold and seven is.
///
/// This is the rule test for `Condition::GraveyardCountAtLeast`, and the
/// boundary is the whole of it: an off-by-one here is a land that is either
/// permanently switched on or permanently switched off, and a test that
/// seeded ten cards would pass against both. So one game, seeded to six,
/// asked, then seeded to seven and asked again — the same board, the same
/// land, one card of difference.
///
/// It is also the regression test for what the missing variant actually
/// did. Cabal Pit shipped the ability **ungated**, which is a strictly
/// better land than the printed one, and Barbarian Ring shipped it not at
/// all. Two opposite wrong answers to one missing sentence, and neither
/// could be caught by `xtask validate`, which reads what a card *says*
/// rather than when the engine offers it.
#[test]
fn cabal_pit_offers_its_ability_only_at_threshold() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9, forest())
        .battlefield(0, &[cabal_pit(), swamp(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pit = on_battlefield(&engine, p0, cabal_pit()).expect("the Pit is in play");

    // The mana first, and that is the whole care this test needs.
    // `legal.abilities` lists what can be paid for out of the pool as it
    // stands, not out of the lands that could fill it — so a negative
    // assertion made with an empty pool is green because the {B} is
    // missing and says nothing at all about the gate.
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are in play");
    tap_mana_except(&mut engine, p0, pit);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the Swamp is tapped and the black mana is floating, so the only \
         thing left that can withhold the ability is the gate"
    );

    seed_graveyard(&mut engine, p0, 6);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority")
    };
    assert!(
        !legal.abilities.contains(&(pit, 1)),
        "six cards is not threshold — the printed gate says seven, and an \
         ungated Cabal Pit is a land nobody printed"
    );

    seed_graveyard(&mut engine, p0, 1);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority")
    };
    assert!(
        legal.abilities.contains(&(pit, 1)),
        "the seventh card is threshold"
    );

    // And it does what it says once it is on.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pit,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("it targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: Vec::new(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "a 1/1 given -2/-2 is a creature with toughness below one (CR 704.5f)"
    );
}
