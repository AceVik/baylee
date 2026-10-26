//! `cards/enchantments/mv_4/death_pit_offering.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Death Pit Offering ({2}{B}{B} Enchantment):
/// "When this enchantment enters, sacrifice all creatures you control.
/// Creatures you control get +2/+2."
#[test]
fn death_pit_offering_sacrifices_your_creatures_on_entry_and_pumps_future_creatures() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .hand(
            0,
            &[
                card_index("d445032b-b243-4203-9c88-a0ac357fb9b5"),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let swamp_ids: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        })
        .collect();
    tap_mana_where(&mut engine, P0, |id| swamp_ids.contains(&id));
    cast_with_floating(
        &mut engine,
        P0,
        card_index("d445032b-b243-4203-9c88-a0ac357fb9b5"),
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none()
            && in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "P0's existing Grizzly Bears was sacrificed when Death Pit Offering entered"
    );
    let opp_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(
        pt(&engine, opp_bears),
        (2, 2),
        "opponent's Grizzly Bears was neither sacrificed nor pumped"
    );

    // Cast Hill Giant afterwards: it gets +2/+2 -> 5/5.
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 4);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P0, index::HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, giant), (5, 5));
}
