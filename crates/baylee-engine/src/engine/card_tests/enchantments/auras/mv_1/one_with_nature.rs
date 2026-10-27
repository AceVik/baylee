//! One with Nature — `{G}` Enchantment — Aura (SCG).
//! Oracle: "Enchant creature. Whenever enchanted creature deals combat damage to a player, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle."

use super::*;

fn one_with_nature() -> CardIndex {
    card_index("e48d3de9-a5e0-4a69-8377-9e3ad5fc5b77")
}

#[test]
fn one_with_nature_searches_basic_land_to_battlefield_tapped_on_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let aura = one_with_nature();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), grizzly_bears()])
        .hand(0, &[aura])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let initial_forests = all_on_battlefield(&engine, p0, forest()).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, aura);
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

    // Attack p1 unblocked with the enchanted Grizzly Bears.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Pick a basic Forest from the library (optional search has min: 0, max: 1).
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let forests_after = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests_after.len(), initial_forests + 1);
    assert!(forests_after.iter().all(|&id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    }));
}
