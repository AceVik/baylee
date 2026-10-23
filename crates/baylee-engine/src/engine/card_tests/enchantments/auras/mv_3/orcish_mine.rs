//! Orcish Mine — `{1}{R}{R}` Enchantment — Aura (Homelands)
//!
//! Enchant land
//! This Aura enters with three ore counters on it.
//! At the beginning of your upkeep and whenever enchanted land becomes tapped,
//! remove an ore counter from this Aura.
//! When the last ore counter is removed from this Aura, destroy enchanted land
//! and this Aura deals 2 damage to that land's controller.
//!
//! (`Coverage::Partial`: ore counter type is not yet in `counters.rs`; uses
//! `counters::MINING` for the three counters, upkeep/tap removal, and
//! last-counter land destruction and 2 damage to its controller.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn orcish_mine() -> CardIndex {
    card_index("a6bfa03e-6317-4550-960b-fbd2a30e521f")
}

#[test]
fn orcish_mine_removes_counters_on_tap_and_upkeep_then_destroys_land_for_two_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[orcish_mine()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let aura = in_hand(&engine, p0, orcish_mine()).unwrap();
    let forests = all_on_battlefield(&engine, p1, forest());
    let (land, other_land) = (forests[0], forests[1]);
    let p0_life_before = engine.state().players[0].life;
    let p1_life_before = engine.state().players[1].life;

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: aura })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let mine = on_battlefield(&engine, p0, orcish_mine()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MINING),
        3
    );

    // Pass to P1's main phase. P1's upkeep removed nothing, and tapping the
    // other Forest removes nothing; tapping the enchanted one removes the 1st
    // counter -> 2 left.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: other_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MINING),
        3,
        "a land the Aura is not on tapped, and the opponent's upkeep is not yours"
    );
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MINING),
        2
    );

    // Pass through P0's next upkeep (removes 2nd counter -> 1 left) to P1's next main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine
            .state()
            .object(mine)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::MINING),
        1
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });

    // Tap the enchanted Forest again to remove the 3rd (last) counter -> destroys Forest and deals 2 damage to P1!
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(land).unwrap().zone,
        Zone::Graveyard,
        "the enchanted Forest was destroyed"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, forest()),
        vec![other_land],
        "the other Forest is untouched"
    );
    assert_eq!(engine.state().players[1].life, p1_life_before - 2);
    assert_eq!(
        engine.state().players[0].life,
        p0_life_before,
        "the land's controller takes it, not the Aura's"
    );
}
