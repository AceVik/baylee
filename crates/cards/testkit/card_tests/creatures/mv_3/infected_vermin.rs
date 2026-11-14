//! `cards/creatures/mv_3/infected_vermin.rs`, played.
//!
//! Oracle:
//! - "{2}{B}: This creature deals 1 damage to each creature and each player."
//! - "Threshold — {3}{B}: This creature deals 3 damage to each creature and each
//!   player. Activate only if seven or more cards are in your graveyard."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infected_vermin() -> CardIndex {
    card_index("448396b8-b691-457a-8569-1282c3fca65a")
}

#[test]
fn infected_vermin_deals_one_damage_and_three_at_threshold() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                infected_vermin(),
                index::GRIZZLY_BEARS,
                index::SHIVAN_DRAGON,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[index::SHOCK, infected_vermin()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 5 seeded + Shock on Shivan Dragon (5/5 -> 2 damage) = 6 cards (< 7).
    seed_graveyard(&mut engine, P0, 5);
    let dragon = on_battlefield(&engine, P0, index::SHIVAN_DRAGON).unwrap();
    let v1 = on_battlefield(&engine, P0, infected_vermin()).unwrap();
    assert_eq!(pt(&engine, v1), (1, 1));
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .unwrap();
    cast_with_floating(&mut engine, P0, index::SHOCK);
    aim_at(&mut engine, P0, dragon);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        6
    );
    assert!(ability_offered(&engine, v1, 0));
    assert!(!ability_offered(&engine, v1, 1), "Threshold needs 7 cards");

    // Activate ability 0 ({2}{B}): 1 damage to each creature and each player.
    let swamps = all_on_battlefield(&engine, P0, swamp());
    for &s in &swamps[..3] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    activate(&mut engine, P0, infected_vermin(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().object(dragon).unwrap().damage, 3);
    assert!(
        in_graveyard(&engine, P0, infected_vermin()).is_some(),
        "1/1 Vermin #1 died to its own 1 damage, raising graveyard to 7"
    );

    // Cast Vermin #2 ({2}{B}) and advance to P0's next turn so lands untap and damage clears.
    for &s in &swamps[3..] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, infected_vermin());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));

    let v2 = on_battlefield(&engine, P0, infected_vermin()).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        7
    );
    assert!(ability_offered(&engine, v2, 1));

    // Activate Threshold ability 1 ({3}{B}): 3 damage to each creature and each player.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, infected_vermin(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(engine.state().players[1].life, 16);
    assert!(on_battlefield(&engine, P0, infected_vermin()).is_none());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    let _ = bears;
    assert_eq!(engine.state().object(dragon).unwrap().damage, 3);
    assert!(on_battlefield(&engine, P0, index::SHIVAN_DRAGON).is_some());
}
