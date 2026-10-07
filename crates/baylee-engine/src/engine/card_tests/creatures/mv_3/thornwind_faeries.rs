//! `cards/creatures/mv_3/thornwind_faeries.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thornwind Faeries is a {1}{U}{U} 1/1 Faerie with flying, and its whole
/// printed text is "{T}: This creature deals 1 damage to any target."
/// The scenario casts it off three Islands, walks a full turn cycle so the
/// tap symbol it prints is actually payable (CR 302.6), and then aims the
/// ping at the opponent: the life total moves by one, the Faerie is tapped
/// as the last step of the activation (CR 601.2h), and the Elf standing
/// across the table is untouched — so "any target" reached the player that
/// was named and not a permanent.
#[test]
fn thornwind_faeries_taps_to_deal_one_damage_to_the_player_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[thornwind_faeries()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thornwind_faeries());
    pass_until(&mut engine, stack_is_empty);
    let faerie = on_battlefield(&engine, p0, thornwind_faeries()).expect("the Faerie resolved");
    assert_eq!(pt(&engine, faerie), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, faerie).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert!(
        !is_tapped(&engine, faerie),
        "it enters untapped, so the {{T}} it prints is still its to spend"
    );

    // A creature that arrived this turn cannot pay a {T} cost (CR 302.6), so
    // the ping is only offered after a whole turn cycle — in which the three
    // Islands and the Faerie alike come back in the untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, faerie),
        "the untap step stood it back up"
    );

    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    activate(&mut engine, p0, thornwind_faeries(), 0);
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
        options.contains(&theirs),
        "CR 115.4: the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "and players are counted in the same choice: {player_options:?}"
    );
    assert!(
        !is_tapped(&engine, faerie),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so the tap symbol has not been spent while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target for `any target`");

    assert!(is_tapped(&engine, faerie), "the {{T}} was the price");
    assert!(
        !stack_is_empty(&engine),
        "and dealing damage is no mana ability"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" — one, to the player that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat that was aimed at"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf nobody aimed at is untouched: the damage went to the player \
         and not to a permanent on the board"
    );
    assert!(
        on_battlefield(&engine, p0, thornwind_faeries()).is_some(),
        "an activated ability costs the creature nothing but its tap"
    );
}
