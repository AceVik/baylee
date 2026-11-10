//! `cards/artifacts/equipment/mv_3/sword_of_hearth_and_home.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// ---- Abilities no test had fired (L4 sweep, 2026-10-01) ----

/// Sword of Hearth and Home: "Whenever equipped creature deals combat damage
/// to a player, exile up to one target creature you own, then search your
/// library for a basic land card. Put both cards onto the battlefield under
/// your control, then shuffle." Equipped with it the Bears hit for 4 (+2/+2);
/// the Elves are blinked and a Forest comes from the library.
#[test]
fn sword_of_hearth_and_home_blinks_a_creature_and_fetches_a_basic_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let sword = card_index("913e6182-706a-4872-8c8a-e146b0ae0738");
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut engine = Duel::new(2601, island())
        .battlefield(0, &[sword, bears, llanowar_elves(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let b = on_battlefield(&engine, p0, bears).expect("bears");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    let lands_before = lands_of(&engine, p0).len();
    let lib = library_size(&engine, p0);
    let life = engine.state().players[1].life;
    tap_mana_where(&mut engine, p0, |id| id != elf);

    activate(&mut engine, p0, sword, 4);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("equip asks for a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&b));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![b] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, b), (4, 4), "equipped: +2/+2");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(elf)
        .expect("on the table")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the attack question")
    };
    let defender = defenders.into_iter().next().expect("a defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(b, defender)],
            },
        )
        .unwrap();
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&elf), "a creature I own is a target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the search")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(1).collect(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, life - 4, "combat damage");
    assert_eq!(
        library_size(&engine, p0),
        lib - 1,
        "a land left the library"
    );
    assert_eq!(lands_of(&engine, p0).len(), lands_before + 1, "and arrived");
    let back = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves came back");
    assert!(
        !is_tapped(&engine, back),
        "a tapped Elf that was exiled returns as a new, untapped object (CR 400.7)"
    );
    let _ = p1;
}
