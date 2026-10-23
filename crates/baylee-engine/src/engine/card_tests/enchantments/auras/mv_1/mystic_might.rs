//! `cards/enchantments/auras/mv_1/mystic_might.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mystic_might() -> CardIndex {
    card_index("967eca77-d273-4fc6-97d1-df7d8cc6ff59")
}

/// Mystic Might (967eca77-d273-4fc6-97d1-df7d8cc6ff59) — {U} Enchantment — Aura:
/// - "Enchant land you control"
/// - "Cumulative upkeep {1}{U}"
/// - "Enchanted land has '{T}: Target creature gets +2/+2 until end of turn.'"
#[test]
fn mystic_might_enchants_only_your_land_grants_tap_pump_and_has_cumulative_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), forest(), island(), grizzly_bears()])
        .battlefield(1, &[island()])
        .hand(0, &[mystic_might()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_forest = on_battlefield(&engine, P0, forest()).unwrap();
    let their_island = on_battlefield(&engine, P1, island()).unwrap();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    // Cast Mystic Might: only P0's lands are legal targets ("Enchant land you control").
    let first_island = on_battlefield(&engine, P0, island()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == first_island);
    cast_with_floating(&mut engine, P0, mystic_might());
    let options = aim_at(&mut engine, P0, my_forest);
    assert!(
        !options.contains(&their_island),
        "opponent's land is excluded by 'Enchant land you control'"
    );
    pass_until(&mut engine, stack_is_empty);

    // Only the enchanted land has the ability.
    let other_island = all_on_battlefield(&engine, P0, island())
        .into_iter()
        .find(|&id| id != first_island)
        .unwrap();
    assert!(
        ability_offered(&engine, my_forest, crate::choice::GRANTED_ABILITY),
        "the enchanted land has the granted ability"
    );
    assert!(
        !ability_offered(&engine, other_island, crate::choice::GRANTED_ABILITY),
        "a land that is not enchanted does not"
    );

    // Enchanted Forest has "{T}: Target creature gets +2/+2 until end of turn".
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_forest,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bears),
        (4, 4),
        "enchanted land's granted {{T}} ability gives target creature +2/+2 until end of turn"
    );

    // P0's first upkeep with Mystic Might (turn 3): cumulative upkeep {1}{U} paid.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(on_battlefield(&engine, P0, mystic_might()).is_some());

    // P0's second upkeep (turn 5): 2nd age counter placed; P0 declines cumulative upkeep.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, P0, mystic_might()).is_none()
            && in_graveyard(&engine, P0, mystic_might()).is_some(),
        "Mystic Might is sacrificed when cumulative upkeep is declined"
    );
}
