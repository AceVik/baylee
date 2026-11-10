//! `cards/sorceries/mv_3/crippling_fatigue.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn crippling_fatigue() -> CardIndex {
    card_index("d87190d0-bda3-4ad9-84b2-019f751999ce")
}

/// Both casts give exactly -2/-2 to one creature. The flashback cast pays
/// {1}{B} and 3 life instead of {1}{B}{B}, and exiles after resolving.
#[test]
fn crippling_fatigue_pumps_one_creature_and_flashes_back_for_mana_and_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[index::HILL_GIANT, index::GRIZZLY_BEARS])
        .hand(0, &[crippling_fatigue()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, crippling_fatigue());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .is_err(),
        "a player is not a creature target"
    );
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (1, 1));
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(life_of(&engine, p0), 20);
    let fatigue = in_graveyard(&engine, p0, crippling_fatigue()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: fatigue })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine
            .apply(p0, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
    aim_at(&mut engine, p0, bears);
    assert_eq!(life_of(&engine, p0), 17, "the life is paid while casting");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(pt(&engine, giant), (1, 1));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(fatigue).unwrap().zone, Zone::Exile);
}

/// The power/toughness modification ends with the caster's turn.
#[test]
fn crippling_fatigue_pump_expires_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 3])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[crippling_fatigue()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, crippling_fatigue());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (1, 1));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, giant), (3, 3));
}
