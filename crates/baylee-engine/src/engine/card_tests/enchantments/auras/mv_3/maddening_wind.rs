//! `cards/enchantments/auras/mv_3/maddening_wind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn maddening_wind() -> CardIndex {
    card_index("c9798915-3f9e-466b-bbd6-f4e126bdb67d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Maddening Wind (c9798915-3f9e-466b-bbd6-f4e126bdb67d) — {2}{G} Enchantment — Aura:
/// - "Enchant creature"
/// - "Cumulative upkeep {G}"
/// - "At the beginning of the upkeep of enchanted creature's controller, this Aura deals 2 damage to that player."
#[test]
fn maddening_wind_deals_two_damage_on_enchanted_creature_controllers_upkeep_and_has_cumulative_upkeep()
 {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[maddening_wind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, maddening_wind());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's upkeep (turn 2): Maddening Wind deals 2 damage to P1 (enchanted creature's controller).
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 20);

    // Advance to P0's upkeep (turn 3): cumulative upkeep {G} puts 1 age counter on Maddening Wind.
    // Branch 1: P0 pays {G}.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana_but(&mut engine, P0, None);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, maddening_wind()).is_some(),
        "Maddening Wind stays on the battlefield when cumulative upkeep is paid"
    );

    // On P1's next upkeep (turn 4), P1 takes another 2 damage (down to 16).
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);

    // Advance to P0's second upkeep (turn 5): 2nd age counter placed; P0 declines cumulative upkeep.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, maddening_wind()).is_none()
            && in_graveyard(&engine, P0, maddening_wind()).is_some(),
        "Maddening Wind is sacrificed when cumulative upkeep is declined"
    );
}
