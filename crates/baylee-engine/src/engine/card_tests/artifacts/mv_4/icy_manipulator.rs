//! `cards/artifacts/mv_4/icy_manipulator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Icy Manipulator — {4} artifact: "{1}, {T}: Tap target artifact, creature,
/// or land." The filter is the whole card, so the board carries all three of
/// its words on both sides of the table and one permanent that is none of
/// them — an Exploration, an enchantment a bare "target permanent" would have
/// offered and this must decline. Five Forests pay the cast and leave exactly
/// the {1} the ability charges, so the offer is claimed off a pool that really
/// holds it, and the target is named before the tap and the mana go
/// (CR 601.2c, then CR 601.2h): the Manipulator is still standing and the
/// green still floating while the target question is open, and the land stays
/// untapped until the ability actually resolves.
#[test]
#[allow(clippy::too_many_lines)]
fn icy_manipulator_taps_an_artifact_creature_or_land_but_never_an_enchantment() {
    fn icy_manipulator() -> CardIndex {
        card_index("3608f1f7-8dc5-4dd1-ae91-c830e1de9529")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![forest(); 5];
    board.extend([llanowar_elves(), exploration()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[forest(), quiet_artifact()])
        .hand(0, &[icy_manipulator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests, and the Elf named as the printing kept back: it is the
    // creature this test reads afterwards, and a source tapped for mana is a
    // source whose status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Forests, five green, and the untapped Elf gave nothing"
    );
    cast_with_floating(&mut engine, p0, icy_manipulator());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let manipulator =
        on_battlefield(&engine, p0, icy_manipulator()).expect("the Manipulator resolved");
    assert!(
        !is_tapped(&engine, manipulator),
        "an artifact enters untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}} is spent and exactly the {{1}} the ability charges is left"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — so the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(manipulator, 0)),
        "with {{1}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let chant = on_battlefield(&engine, p0, exploration()).expect("the Exploration is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    activate(&mut engine, p0, icy_manipulator(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact, creature, or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&elf),
        "a creature, on this side of the table: {options:?}"
    );
    assert!(
        options.contains(&their_rock),
        "an artifact, whichever seat controls it: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "and a land, so all three words of the filter are read: {options:?}"
    );
    assert!(
        !options.contains(&chant),
        "an enchantment is a permanent and none of the three: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, manipulator),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the land was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, manipulator),
        "{{T}} is part of the price and is paid with the rest"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a permanent is no mana ability, so the ability is waiting"
    );
    assert!(
        !is_tapped(&engine, their_land),
        "and the effect has not resolved yet: the target is still where it was"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, their_land),
        "\"Tap target artifact, creature, or land\" — the land that was named"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and nothing else: the creature beside it was left alone"
    );
    assert!(
        !is_tapped(&engine, their_rock),
        "nor the artifact across the table"
    );
    assert!(
        !is_tapped(&engine, chant),
        "nor the enchantment the filter declined"
    );
    assert!(
        on_battlefield(&engine, p0, icy_manipulator()).is_some(),
        "the price was a tap and no sacrifice, so the Manipulator stays standing"
    );
}
