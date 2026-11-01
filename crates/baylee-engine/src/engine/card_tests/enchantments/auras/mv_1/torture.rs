//! Torture — `{B}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! `{1}{B}`: Put a -1/-1 counter on enchanted creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn torture() -> CardIndex {
    card_index("389b6d77-47ab-43a3-99d1-dcc762939cc4")
}

#[test]
fn torture_puts_minus_one_minus_one_counters_on_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[torture()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let torture_card = in_hand(&engine, p0, torture()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let opposing_swamp = on_battlefield(&engine, p1, swamp()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: torture_card })
        .unwrap();
    // "Enchant creature": a land is not a legal target.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![opposing_swamp],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let torture_bf = on_battlefield(&engine, p0, torture()).unwrap();

    // Activate {1}{B} twice: Grizzly Bears gets two -1/-1 counters and dies as 0/0.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: torture_bf,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (1, 1));
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "only the enchanted creature gets the counter"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: torture_bf,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Graveyard,
        "Grizzly Bears died at 0/0 after two -1/-1 counters from Torture"
    );
    assert_eq!(pt(&engine, ogre), (2, 2));
}
