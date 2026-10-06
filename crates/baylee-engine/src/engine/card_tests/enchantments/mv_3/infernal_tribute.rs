//! `cards/enchantments/mv_3/infernal_tribute.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Infernal Tribute is an enchantment under `Coverage::Implemented` costing {B}{B}{B} with an activated card-draw ability.
/// Paying {2} and sacrificing a nontoken permanent draws a card.
/// The sacrifice is requested as a cost prompt with `ChoicePrompt::CostSacrifice` where controlled nontoken permanents are offered.
/// Upon resolution, the sacrificed permanent is in the graveyard and its controller draws one card from their library.
#[test]
fn infernal_tribute_sacrifices_nontoken_permanent_to_draw_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infernal_tribute(), swamp(), swamp(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let tribute = on_battlefield(&engine, p0, infernal_tribute())
        .expect("Infernal Tribute is on battlefield");
    let elf =
        on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves is on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == tribute && *idx == 0),
        "ability requiring {{2}} is not offered on an empty mana pool"
    );

    // The Elves are kept back: their own `{T}: Add {G}` is a third mana
    // route, and they are the permanent the sacrifice is about to name.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps produce two mana"
    );

    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, infernal_tribute(), 0);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "controller chooses what to sacrifice");
    assert_eq!((min, max), (1, 1), "exactly one permanent sacrificed");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "choice is flagged as a cost sacrifice"
    );
    assert!(
        options.contains(&elf),
        "controlled nontoken creature is on the menu: {options:?}"
    );
    assert!(
        options.contains(&tribute),
        "Infernal Tribute itself is a nontoken permanent and on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("sacrificing the Elf is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "sacrificed creature is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "sacrificed creature left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, infernal_tribute()).is_some(),
        "Infernal Tribute remains on the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "one card was drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drawn card arrived in hand"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the floating mana was spent"
    );
}
