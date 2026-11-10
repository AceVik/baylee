//! Lethal Vapors — `{2}{B}{B}` Enchantment (SCG).
//! Oracle: "Whenever a creature enters, destroy it. {0}: Destroy this enchantment. You skip your next turn. Any player may activate this ability."

use super::*;

fn lethal_vapors() -> CardIndex {
    card_index("9fe9650d-6c9b-4988-9ea4-79ba0d55a77b")
}

#[test]
fn lethal_vapors_destroys_entering_creatures_and_any_player_may_activate_destroy_self() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let vapors = lethal_vapors();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), forest(), forest()])
        .hand(0, &[vapors, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamps = all_on_battlefield(&engine, p0, swamp());
    let forests = all_on_battlefield(&engine, p0, forest());

    // Cast Lethal Vapors for {2}{B}{B}.
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, vapors);
    pass_until(&mut engine, stack_is_empty);
    let vapors_id = on_battlefield(&engine, p0, vapors).unwrap();

    // Casting Grizzly Bears causes it to enter and immediately get destroyed by Lethal Vapors.
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());

    // Pass priority to p1; p1 (the opponent) activates Lethal Vapors's {0} ability (index 1).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: vapors_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, vapors).is_none());
    assert!(in_graveyard(&engine, p0, vapors).is_some());
    assert_eq!(engine.state().players[1].turns_to_skip, 1);
}
