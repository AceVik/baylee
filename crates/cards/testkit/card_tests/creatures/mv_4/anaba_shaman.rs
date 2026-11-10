//! `cards/creatures/mv_4/anaba_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Anaba Shaman is a {3}{R} 2/2 whose whole printed line is "{R}, {T}: This
/// creature deals 1 damage to any target." CR 115.4's "any target" is the word
/// worth playing: one choice carries the object options *and* the player
/// options, so the board stands a 1/1 across the table to be named beside both
/// seats to be offered, and the damage has to land on the one that was answered
/// and nowhere else. Both halves of the price are read on both sides of the
/// target question — still in the pool and the Shaman still standing while the
/// choice is open (CR 601.2c before CR 601.2h), spent and tapped once it is
/// answered — and the four Mountains are tapped first because
/// `legal.abilities` is filtered through `can_afford`, which reads the pool
/// and not the untapped lands.
#[test]
fn anaba_shaman_pays_red_and_its_tap_for_one_damage_to_any_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                anaba_shaman(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shaman = on_battlefield(&engine, p0, anaba_shaman()).expect("the Shaman is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("a 1/1 across the table");
    assert_eq!(pt(&engine, shaman), (2, 2), "the body the card prints");

    // Mana before the claim: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped Mountains.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and the Shaman makes no mana of its own"
    );
    assert!(
        !is_tapped(&engine, shaman),
        "nothing has tapped the Shaman yet"
    );

    activate(&mut engine, p0, anaba_shaman(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so both halves
    // of the price are read here rather than before the answer.
    assert!(
        !is_tapped(&engine, shaman),
        "the {{T}} is the last step of the activation"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{R}} is still floating for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    assert!(is_tapped(&engine, shaman), "{{T}} was paid by the Shaman");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "one red paid the {{R}}"
    );
    assert!(
        !stack_is_empty(&engine),
        "dealing damage is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the player \
         whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, anaba_shaman()).is_some(),
        "an activated ability costs the Shaman nothing but its tap"
    );
}
