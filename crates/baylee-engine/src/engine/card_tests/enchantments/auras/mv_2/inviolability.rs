//! `cards/enchantments/auras/mv_2/inviolability.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "3f336198-2425-447a-88b4-9be21ef7e90a"
fn inviolability() -> CardIndex {
    card_index("3f336198-2425-447a-88b4-9be21ef7e90a")
}

/// `Inviolability` (`Coverage::Generated`):
/// "Enchant creature. Prevent all damage that would be dealt to enchanted creature."
///
/// Verifies that `Inviolability` prevents all noncombat and combat damage that
/// would be dealt to the enchanted creature, while leaving unenchanted creatures
/// vulnerable to damage.
#[test]
fn inviolability_prevents_all_damage_to_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9501, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                mountain(),
                mountain(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[inviolability(), lightning_bolt(), index::SHOCK])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Llanowar Elves");
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).expect("P1 Hill Giant");

    // Enchant Grizzly Bears with Inviolability.
    let plains_cards = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| plains_cards.contains(&id));
    cast_with_floating(&mut engine, p0, inviolability());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Lightning Bolt (3 damage) at the enchanted Grizzly Bears (2/2) -> all 3 damage prevented.
    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "enchanted Grizzly Bears survives Lightning Bolt because all damage is prevented"
    );
    assert_eq!(engine.state().arena.get(bear).unwrap().damage, 0);

    // Unenchanted Llanowar Elves still dies to Shock.
    cast_with_floating(&mut engine, p0, index::SHOCK);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "unenchanted creature still takes lethal damage"
    );

    // On P1's turn, P1 attacks with 3/3 Hill Giant and P0 blocks with 2/2 enchanted Grizzly Bears.
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, giant)],
            },
        )
        .unwrap();
    assert!(walk_to_own_main(&mut engine, p0));

    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "enchanted Grizzly Bears survives blocking a 3/3 Hill Giant"
    );
}
