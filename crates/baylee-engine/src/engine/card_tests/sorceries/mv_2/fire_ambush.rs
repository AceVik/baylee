//! `cards/sorceries/mv_2/fire_ambush.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Ambush — {1}{R} sorcery: "Fire Ambush deals 3 damage to any target."
///
/// "Any target" is the whole card (CR 115.4), so the board is built to tell
/// the two halves of that word apart: a creature under each seat shows up in
/// the object list and both players in the player list, and the damage is then
/// aimed at the opponent's creature rather than at the seat whose board it
/// stands on. Three damage to a printed 1/1 is lethal (CR 704.5g) while the
/// opponent's life total stays where it was — which is what says the number
/// landed on the creature and not on the player — and p0's own Elf, untouched,
/// is the control that the spell hit what it was aimed at and nothing else.
#[test]
fn fire_ambush_deals_three_damage_to_the_creature_it_targets_and_not_to_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fire_ambush()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "a 1/1 for three damage to kill"
    );

    // Two Mountains pay {1}{R}. The Elf is named as the thing kept back: it
    // prints its own {T}: Add {G}, so `tap_all_mana` would have spent it too
    // and every mana number below would be a claim about a third source.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains tapped, and the Elf contributed nothing"
    );
    let life_before_p1 = engine.state().players[1].life;
    cast_with_floating(&mut engine, p0, fire_ambush());

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
    assert_eq!(player, p0, "the caster is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"any target\" reaches either side of the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4 counts players in the same choice: {player_options:?}"
    );
    // The card is still in hand: `cast_wizard` asks every question
    // CR 601.2b–h poses and moves it to the stack last, at CR 601.2i, so the
    // announcement is atomic from the outside. What this reads instead is
    // the thing the assertion was really about — the damage has not been
    // dealt while the question is open (CR 608.2b).
    assert!(
        in_hand(&engine, p0, fire_ambush()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before_p1,
        "and nothing has been dealt while the target is being named"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered was chosen");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the target is still there while the sorcery sits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to its \
         controller"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool the two Mountains filled"
    );
    assert!(
        in_graveyard(&engine, p0, fire_ambush()).is_some(),
        "and the sorcery is in its owner's graveyard once it has resolved"
    );
}
